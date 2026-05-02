use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use vaexcore_core::{
    new_id, now_utc, AppSettings, Marker, MediaProfile, MediaProfileInput, PlatformKind,
    ProfilesSnapshot, RecordingContainer, Resolution, SecretRef, SecretStore, SecretStoreError,
    SensitiveString, StreamDestination, StreamDestinationInput,
};
use vaexcore_platforms::apply_platform_defaults;

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

    fn migrate(&self) -> Result<(), StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        connection.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

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
        Ok(())
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
}
