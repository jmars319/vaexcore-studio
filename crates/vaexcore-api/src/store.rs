use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use vaexcore_core::{
    new_id, now_utc, AppSettings, AuditLogEntry, Marker, MediaProfile, MediaProfileInput,
    PlatformKind, ProfilesSnapshot, RecordingContainer, Resolution, SecretRef, SecretStore,
    SecretStoreError, SensitiveString, StreamDestination, StreamDestinationInput,
};
use vaexcore_platforms::apply_platform_defaults;

const CURRENT_SCHEMA_VERSION: u32 = 2;
const AUDIT_LOG_LIMIT: usize = 200;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid persisted value: {0}")]
    InvalidValue(String),
    #[error("secret store error: {0}")]
    SecretStore(#[from] SecretStoreError),
}

#[derive(Clone)]
pub struct ProfileStore {
    connection: Arc<Mutex<Connection>>,
}

impl ProfileStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                StoreError::InvalidValue(format!(
                    "failed to create database directory '{}': {error}",
                    parent.display()
                ))
            })?;
        }

        let connection = Connection::open(path)?;
        let store = Self {
            connection: Arc::new(Mutex::new(connection)),
        };
        store.migrate()?;
        store.seed_defaults()?;
        Ok(store)
    }

    pub fn open_memory() -> Result<Self, StoreError> {
        let connection = Connection::open_in_memory()?;
        let store = Self {
            connection: Arc::new(Mutex::new(connection)),
        };
        store.migrate()?;
        store.seed_defaults()?;
        Ok(store)
    }

    pub fn profiles_snapshot(&self) -> Result<ProfilesSnapshot, StoreError> {
        Ok(ProfilesSnapshot {
            recording_profiles: self.list_recording_profiles()?,
            stream_destinations: self.list_stream_destinations()?,
        })
    }

    pub fn initialize_app_settings(&self, seed: AppSettings) -> Result<AppSettings, StoreError> {
        if let Some(settings) = self.read_app_settings()? {
            return Ok(settings);
        }

        self.save_app_settings(seed)
    }

    pub fn app_settings(&self) -> Result<AppSettings, StoreError> {
        Ok(self.read_app_settings()?.unwrap_or_default())
    }

    pub fn save_app_settings(&self, mut settings: AppSettings) -> Result<AppSettings, StoreError> {
        settings.api_host = settings.api_host.trim().to_string();
        settings.api_token = settings
            .api_token
            .map(|token| token.trim().to_string())
            .filter(|token| !token.is_empty());
        settings.log_level = settings.log_level.trim().to_ascii_lowercase();
        settings.validate().map_err(StoreError::InvalidValue)?;

        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        connection.execute(
            "INSERT INTO app_settings (id, value_json, updated_at)
             VALUES ('app', ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET
               value_json = excluded.value_json,
               updated_at = excluded.updated_at",
            params![serde_json::to_string(&settings)?, now_utc().to_rfc3339()],
        )?;

        Ok(settings)
    }

    pub fn insert_recording_profile(
        &self,
        input: MediaProfileInput,
    ) -> Result<MediaProfile, StoreError> {
        let profile = MediaProfile::from_input(input);
        self.insert_recording_profile_model(&profile)?;
        Ok(profile)
    }

    pub fn update_recording_profile(
        &self,
        id: &str,
        input: MediaProfileInput,
    ) -> Result<Option<MediaProfile>, StoreError> {
        let Some(existing) = self.recording_profile_by_id(Some(id))? else {
            return Ok(None);
        };

        let profile = MediaProfile {
            id: existing.id,
            name: input.name,
            output_folder: input.output_folder,
            filename_pattern: input.filename_pattern,
            container: input.container,
            resolution: input.resolution,
            framerate: input.framerate,
            bitrate_kbps: input.bitrate_kbps,
            encoder_preference: input.encoder_preference,
            created_at: existing.created_at,
            updated_at: now_utc(),
        };

        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let changed = connection.execute(
            "UPDATE recording_profiles
             SET name = ?2,
                 output_folder = ?3,
                 filename_pattern = ?4,
                 container = ?5,
                 width = ?6,
                 height = ?7,
                 framerate = ?8,
                 bitrate_kbps = ?9,
                 encoder_preference_json = ?10,
                 updated_at = ?11
             WHERE id = ?1",
            params![
                profile.id,
                profile.name,
                profile.output_folder,
                profile.filename_pattern,
                profile.container.as_str(),
                profile.resolution.width,
                profile.resolution.height,
                profile.framerate,
                profile.bitrate_kbps,
                serde_json::to_string(&profile.encoder_preference)?,
                profile.updated_at.to_rfc3339(),
            ],
        )?;

        Ok((changed > 0).then_some(profile))
    }

    pub fn delete_recording_profile(&self, id: &str) -> Result<bool, StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let changed = connection.execute("DELETE FROM recording_profiles WHERE id = ?1", [id])?;
        Ok(changed > 0)
    }

    pub fn list_recording_profiles(&self) -> Result<Vec<MediaProfile>, StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let mut statement = connection.prepare(
            "SELECT id, name, output_folder, filename_pattern, container, width, height, framerate,
                    bitrate_kbps, encoder_preference_json, created_at, updated_at
             FROM recording_profiles
             ORDER BY created_at ASC",
        )?;

        let rows = statement.query_map([], |row| {
            let container: String = row.get(4)?;
            let encoder_json: String = row.get(9)?;
            let created_at: String = row.get(10)?;
            let updated_at: String = row.get(11)?;

            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                container,
                row.get::<_, u32>(5)?,
                row.get::<_, u32>(6)?,
                row.get::<_, u32>(7)?,
                row.get::<_, u32>(8)?,
                encoder_json,
                created_at,
                updated_at,
            ))
        })?;

        rows.map(|row| {
            let (
                id,
                name,
                output_folder,
                filename_pattern,
                container,
                width,
                height,
                framerate,
                bitrate_kbps,
                encoder_json,
                created_at,
                updated_at,
            ) = row?;

            Ok(MediaProfile {
                id,
                name,
                output_folder,
                filename_pattern,
                container: RecordingContainer::try_from(container.as_str())
                    .map_err(StoreError::InvalidValue)?,
                resolution: Resolution { width, height },
                framerate,
                bitrate_kbps,
                encoder_preference: serde_json::from_str(&encoder_json)?,
                created_at: parse_time(&created_at)?,
                updated_at: parse_time(&updated_at)?,
            })
        })
        .collect()
    }

    pub fn insert_stream_destination(
        &self,
        input: StreamDestinationInput,
    ) -> Result<StreamDestination, StoreError> {
        let mut input = apply_platform_defaults(input);
        let stream_key_ref = input
            .stream_key
            .as_ref()
            .filter(|secret| !secret.is_empty())
            .map(|secret| self.put_secret("stream_destination", secret))
            .transpose()?;

        input.stream_key = None;
        let destination = StreamDestination::from_input(input, stream_key_ref);
        self.insert_stream_destination_model(&destination)?;
        Ok(destination)
    }

    pub fn update_stream_destination(
        &self,
        id: &str,
        input: StreamDestinationInput,
    ) -> Result<Option<StreamDestination>, StoreError> {
        let Some(existing) = self.stream_destination_by_id_any(id)? else {
            return Ok(None);
        };

        let mut input = apply_platform_defaults(input);
        let old_secret_ref = existing.stream_key_ref.clone();
        let stream_key_ref = input
            .stream_key
            .as_ref()
            .filter(|secret| !secret.is_empty())
            .map(|secret| self.put_secret("stream_destination", secret))
            .transpose()?
            .or_else(|| existing.stream_key_ref.clone());

        input.stream_key = None;
        let destination = StreamDestination {
            id: existing.id,
            name: input.name,
            platform: input.platform,
            ingest_url: input.ingest_url.unwrap_or_default(),
            stream_key_ref,
            enabled: input.enabled.unwrap_or(existing.enabled),
            created_at: existing.created_at,
            updated_at: now_utc(),
        };

        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let changed = connection.execute(
            "UPDATE stream_destinations
             SET name = ?2,
                 platform = ?3,
                 ingest_url = ?4,
                 stream_key_ref_provider = ?5,
                 stream_key_ref_id = ?6,
                 enabled = ?7,
                 updated_at = ?8
             WHERE id = ?1",
            params![
                destination.id,
                destination.name,
                destination.platform.as_str(),
                destination.ingest_url,
                destination
                    .stream_key_ref
                    .as_ref()
                    .map(|secret| &secret.provider),
                destination.stream_key_ref.as_ref().map(|secret| &secret.id),
                destination.enabled,
                destination.updated_at.to_rfc3339(),
            ],
        )?;
        drop(connection);

        if changed > 0 && old_secret_ref != destination.stream_key_ref {
            self.delete_secret_ref(old_secret_ref.as_ref())?;
        }

        Ok((changed > 0).then_some(destination))
    }

    pub fn delete_stream_destination(&self, id: &str) -> Result<bool, StoreError> {
        let Some(existing) = self.stream_destination_by_id_any(id)? else {
            return Ok(false);
        };

        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let changed = connection.execute("DELETE FROM stream_destinations WHERE id = ?1", [id])?;
        drop(connection);

        if changed > 0 {
            self.delete_secret_ref(existing.stream_key_ref.as_ref())?;
        }

        Ok(changed > 0)
    }

    pub fn list_stream_destinations(&self) -> Result<Vec<StreamDestination>, StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let mut statement = connection.prepare(
            "SELECT id, name, platform, ingest_url, stream_key_ref_provider, stream_key_ref_id,
                    enabled, created_at, updated_at
             FROM stream_destinations
             ORDER BY created_at ASC",
        )?;

        let rows = statement.query_map([], |row| {
            let platform: String = row.get(2)?;
            let provider: Option<String> = row.get(4)?;
            let secret_id: Option<String> = row.get(5)?;
            let created_at: String = row.get(7)?;
            let updated_at: String = row.get(8)?;

            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                platform,
                row.get::<_, String>(3)?,
                provider,
                secret_id,
                row.get::<_, bool>(6)?,
                created_at,
                updated_at,
            ))
        })?;

        rows.map(|row| {
            let (
                id,
                name,
                platform,
                ingest_url,
                provider,
                secret_id,
                enabled,
                created_at,
                updated_at,
            ) = row?;

            Ok(StreamDestination {
                id,
                name,
                platform: PlatformKind::try_from(platform.as_str())
                    .map_err(StoreError::InvalidValue)?,
                ingest_url,
                stream_key_ref: provider
                    .zip(secret_id)
                    .map(|(provider, id)| SecretRef { provider, id }),
                enabled,
                created_at: parse_time(&created_at)?,
                updated_at: parse_time(&updated_at)?,
            })
        })
        .collect()
    }

    pub fn recording_profile_by_id(
        &self,
        id: Option<&str>,
    ) -> Result<Option<MediaProfile>, StoreError> {
        let profiles = self.list_recording_profiles()?;
        Ok(match id {
            Some(id) => profiles.into_iter().find(|profile| profile.id == id),
            None => profiles.into_iter().next(),
        })
    }

    pub fn stream_destination_by_id(
        &self,
        id: Option<&str>,
    ) -> Result<Option<StreamDestination>, StoreError> {
        let destinations = self.list_stream_destinations()?;
        Ok(match id {
            Some(id) => destinations
                .into_iter()
                .find(|destination| destination.id == id && destination.enabled),
            None => destinations
                .into_iter()
                .find(|destination| destination.enabled),
        })
    }

    fn stream_destination_by_id_any(
        &self,
        id: &str,
    ) -> Result<Option<StreamDestination>, StoreError> {
        Ok(self
            .list_stream_destinations()?
            .into_iter()
            .find(|destination| destination.id == id))
    }

    pub fn create_marker(&self, label: Option<String>) -> Result<Marker, StoreError> {
        let marker = Marker {
            id: new_id("marker"),
            label,
            created_at: now_utc(),
        };

        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        connection.execute(
            "INSERT INTO markers (id, label, created_at) VALUES (?1, ?2, ?3)",
            params![marker.id, marker.label, marker.created_at.to_rfc3339()],
        )?;

        Ok(marker)
    }

    pub fn insert_audit_log_entry(&self, entry: &AuditLogEntry) -> Result<(), StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        connection.execute(
            "INSERT INTO command_audit_log
             (id, request_id, method, path, action, status_code, ok, client_id, client_name, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                &entry.id,
                &entry.request_id,
                &entry.method,
                &entry.path,
                &entry.action,
                entry.status_code,
                entry.ok,
                &entry.client_id,
                &entry.client_name,
                entry.created_at.to_rfc3339(),
            ],
        )?;
        connection.execute(
            "DELETE FROM command_audit_log
             WHERE id NOT IN (
               SELECT id FROM command_audit_log
               ORDER BY created_at DESC
               LIMIT ?1
             )",
            params![AUDIT_LOG_LIMIT as u32],
        )?;
        Ok(())
    }

    pub fn list_audit_log_entries(&self, limit: usize) -> Result<Vec<AuditLogEntry>, StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let mut statement = connection.prepare(
            "SELECT id, request_id, method, path, action, status_code, ok, client_id, client_name, created_at
             FROM command_audit_log
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;

        let rows = statement.query_map(params![limit.min(AUDIT_LOG_LIMIT) as u32], |row| {
            let created_at: String = row.get(9)?;
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, u16>(5)?,
                row.get::<_, bool>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
                created_at,
            ))
        })?;

        rows.map(|row| {
            let (
                id,
                request_id,
                method,
                path,
                action,
                status_code,
                ok,
                client_id,
                client_name,
                created_at,
            ) = row?;

            Ok(AuditLogEntry {
                id,
                request_id,
                method,
                path,
                action,
                status_code,
                ok,
                client_id,
                client_name,
                created_at: parse_time(&created_at)?,
            })
        })
        .collect()
    }

    fn migrate(&self) -> Result<(), StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        connection.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            "#,
        )?;

        connection.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS schema_migrations (
              version INTEGER PRIMARY KEY,
              applied_at TEXT NOT NULL
            );
            "#,
        )?;

        let current_version = schema_version(&connection)?;
        if current_version > CURRENT_SCHEMA_VERSION {
            return Err(StoreError::InvalidValue(format!(
                "database schema version {current_version} is newer than supported version {CURRENT_SCHEMA_VERSION}"
            )));
        }

        if current_version < 1 {
            apply_migration_1(&connection)?;
        }
        if current_version < 2 {
            apply_migration_2(&connection)?;
        }

        Ok(())
    }

    pub fn schema_version(&self) -> Result<u32, StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        schema_version(&connection)
    }

    fn seed_defaults(&self) -> Result<(), StoreError> {
        if self.count("recording_profiles")? == 0 {
            self.insert_recording_profile_model(&MediaProfile::default_local())?;
        }

        if self.count("stream_destinations")? == 0 {
            self.insert_stream_destination(StreamDestinationInput {
                name: "Dry Run RTMP Target".to_string(),
                platform: PlatformKind::CustomRtmp,
                ingest_url: Some("rtmp://localhost/live".to_string()),
                stream_key: None,
                enabled: Some(true),
            })?;
        }

        Ok(())
    }

    fn count(&self, table_name: &str) -> Result<u32, StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let sql = format!("SELECT COUNT(*) FROM {table_name}");
        let count: u32 = connection.query_row(&sql, [], |row| row.get(0))?;
        Ok(count)
    }

    fn insert_recording_profile_model(&self, profile: &MediaProfile) -> Result<(), StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        connection.execute(
            "INSERT INTO recording_profiles
             (id, name, output_folder, filename_pattern, container, width, height, framerate,
              bitrate_kbps, encoder_preference_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                profile.id,
                profile.name,
                profile.output_folder,
                profile.filename_pattern,
                profile.container.as_str(),
                profile.resolution.width,
                profile.resolution.height,
                profile.framerate,
                profile.bitrate_kbps,
                serde_json::to_string(&profile.encoder_preference)?,
                profile.created_at.to_rfc3339(),
                profile.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn insert_stream_destination_model(
        &self,
        destination: &StreamDestination,
    ) -> Result<(), StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        connection.execute(
            "INSERT INTO stream_destinations
             (id, name, platform, ingest_url, stream_key_ref_provider, stream_key_ref_id, enabled,
              created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                destination.id,
                destination.name,
                destination.platform.as_str(),
                destination.ingest_url,
                destination
                    .stream_key_ref
                    .as_ref()
                    .map(|secret| &secret.provider),
                destination.stream_key_ref.as_ref().map(|secret| &secret.id),
                destination.enabled,
                destination.created_at.to_rfc3339(),
                destination.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    fn read_app_settings(&self) -> Result<Option<AppSettings>, StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let value = connection
            .query_row(
                "SELECT value_json FROM app_settings WHERE id = 'app'",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        value
            .map(|value| serde_json::from_str(&value).map_err(StoreError::Json))
            .transpose()
    }

    fn delete_secret_ref(&self, reference: Option<&SecretRef>) -> Result<(), StoreError> {
        let Some(reference) = reference else {
            return Ok(());
        };

        if reference.provider != "local-sqlite" {
            return Ok(());
        }

        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        connection.execute("DELETE FROM secrets WHERE id = ?1", [&reference.id])?;
        Ok(())
    }
}

fn apply_migration_1(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch(
        r#"
            CREATE TABLE IF NOT EXISTS recording_profiles (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              output_folder TEXT NOT NULL,
              filename_pattern TEXT NOT NULL,
              container TEXT NOT NULL,
              width INTEGER NOT NULL,
              height INTEGER NOT NULL,
              framerate INTEGER NOT NULL,
              bitrate_kbps INTEGER NOT NULL,
              encoder_preference_json TEXT NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS stream_destinations (
              id TEXT PRIMARY KEY,
              name TEXT NOT NULL,
              platform TEXT NOT NULL,
              ingest_url TEXT NOT NULL,
              stream_key_ref_provider TEXT,
              stream_key_ref_id TEXT,
              enabled INTEGER NOT NULL,
              created_at TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS secrets (
              id TEXT PRIMARY KEY,
              scope TEXT NOT NULL,
              secret TEXT NOT NULL,
              created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS markers (
              id TEXT PRIMARY KEY,
              label TEXT,
              created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS app_settings (
              id TEXT PRIMARY KEY,
              value_json TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );
            "#,
    )?;
    connection.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
        params![1_u32, now_utc().to_rfc3339()],
    )?;
    Ok(())
}

fn apply_migration_2(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch(
        r#"
            CREATE TABLE IF NOT EXISTS command_audit_log (
              id TEXT PRIMARY KEY,
              request_id TEXT NOT NULL,
              method TEXT NOT NULL,
              path TEXT NOT NULL,
              action TEXT NOT NULL,
              status_code INTEGER NOT NULL,
              ok INTEGER NOT NULL,
              client_id TEXT,
              client_name TEXT,
              created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_command_audit_log_created_at
            ON command_audit_log(created_at DESC);
            "#,
    )?;
    connection.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
        params![2_u32, now_utc().to_rfc3339()],
    )?;
    Ok(())
}

fn schema_version(connection: &Connection) -> Result<u32, StoreError> {
    Ok(connection
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get::<_, u32>(0),
        )
        .optional()?
        .unwrap_or(0))
}

impl SecretStore for ProfileStore {
    fn put_secret(
        &self,
        scope: &str,
        value: &SensitiveString,
    ) -> Result<SecretRef, SecretStoreError> {
        let id = new_id("secret");
        let connection = self
            .connection
            .lock()
            .map_err(|_| SecretStoreError::Store("profile store mutex poisoned".to_string()))?;

        connection
            .execute(
                "INSERT INTO secrets (id, scope, secret, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![id, scope, value.expose_secret(), now_utc().to_rfc3339()],
            )
            .map_err(|error| SecretStoreError::Store(error.to_string()))?;

        Ok(SecretRef::local(id))
    }

    fn get_secret(
        &self,
        reference: &SecretRef,
    ) -> Result<Option<SensitiveString>, SecretStoreError> {
        if reference.provider != "local-sqlite" {
            return Ok(None);
        }

        let connection = self
            .connection
            .lock()
            .map_err(|_| SecretStoreError::Store("profile store mutex poisoned".to_string()))?;

        let secret = connection
            .query_row(
                "SELECT secret FROM secrets WHERE id = ?1",
                params![reference.id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| SecretStoreError::Store(error.to_string()))?;

        Ok(secret.map(SensitiveString::new))
    }
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, StoreError> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| StoreError::InvalidValue(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_settings_round_trip() {
        let store = ProfileStore::open_memory().unwrap();
        let seed = AppSettings {
            api_token: Some("seed-token".to_string()),
            dev_auth_bypass: true,
            ..AppSettings::default()
        };

        let initialized = store.initialize_app_settings(seed.clone()).unwrap();
        assert_eq!(initialized.api_token, Some("seed-token".to_string()));

        let existing = store
            .initialize_app_settings(AppSettings {
                api_token: Some("ignored-token".to_string()),
                ..AppSettings::default()
            })
            .unwrap();
        assert_eq!(existing.api_token, Some("seed-token".to_string()));

        let saved = store
            .save_app_settings(AppSettings {
                api_port: 51288,
                api_token: Some("  updated-token  ".to_string()),
                log_level: "DEBUG".to_string(),
                ..seed
            })
            .unwrap();

        assert_eq!(saved.api_port, 51288);
        assert_eq!(saved.api_token, Some("updated-token".to_string()));
        assert_eq!(saved.log_level, "debug");
        assert_eq!(store.app_settings().unwrap(), saved);
    }

    #[test]
    fn schema_migration_records_current_version() {
        let store = ProfileStore::open_memory().unwrap();

        assert_eq!(store.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn audit_log_round_trip() {
        let store = ProfileStore::open_memory().unwrap();
        let entry = AuditLogEntry {
            id: new_id("audit"),
            request_id: "req_test".to_string(),
            method: "POST".to_string(),
            path: "/recording/start".to_string(),
            action: "recording.start".to_string(),
            status_code: 200,
            ok: true,
            client_id: Some("client-test".to_string()),
            client_name: Some("Test Client".to_string()),
            created_at: now_utc(),
        };

        store.insert_audit_log_entry(&entry).unwrap();

        let entries = store.list_audit_log_entries(10).unwrap();
        assert_eq!(entries, vec![entry]);
    }
}
