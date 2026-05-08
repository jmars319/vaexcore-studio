use std::{
    path::Path,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};
use vaexcore_core::{
    new_id, now_utc, AppSettings, AuditLogEntry, Marker, MediaProfile, MediaProfileInput,
    PlatformKind, ProfileBundle, ProfileBundleImportResult, ProfilesSnapshot, RecordingContainer,
    RecordingHistoryEntry, RecordingSession, Resolution, SceneCollection, SceneCollectionBundle,
    SceneCollectionImportResult, SceneValidationResult, SecretRef, SecretStore, SecretStoreError,
    SensitiveString, StreamDestination, StreamDestinationBundleItem, StreamDestinationInput,
    LOCAL_SQLITE_SECRET_PROVIDER, MACOS_KEYCHAIN_SECRET_PROVIDER,
    WINDOWS_CREDENTIAL_MANAGER_SECRET_PROVIDER,
};
use vaexcore_platforms::apply_platform_defaults;

const CURRENT_SCHEMA_VERSION: u32 = 5;
const AUDIT_LOG_LIMIT: usize = 200;
const MARKER_LIST_LIMIT: usize = 200;
const MACOS_KEYCHAIN_SERVICE: &str = "com.vaexcore.studio.stream-keys";
#[cfg(target_os = "windows")]
const WINDOWS_CREDENTIAL_TARGET_PREFIX: &str = "vaexcore-studio-stream-key";
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

#[derive(Clone, Debug, Default)]
pub struct MarkerFilters {
    pub source_app: Option<String>,
    pub source_event_id: Option<String>,
    pub recording_session_id: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct MarkerCreateInput {
    pub label: Option<String>,
    pub source_app: Option<String>,
    pub source_event_id: Option<String>,
    pub recording_session_id: Option<String>,
    pub media_path: Option<String>,
    pub start_seconds: Option<f64>,
    pub end_seconds: Option<f64>,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkerWriteResult {
    pub marker: Marker,
    pub created: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecretStorageReport {
    pub secure_storage: String,
    pub secret_storage_state: String,
    pub provider: String,
    pub legacy_stream_key_refs: usize,
    pub migration_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SecretBackend {
    LocalSqlite,
    MacosKeychain {
        service: String,
    },
    #[cfg(target_os = "windows")]
    WindowsCredentialManager {
        target_prefix: String,
    },
}

impl SecretBackend {
    fn default_for_platform() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::MacosKeychain {
                service: MACOS_KEYCHAIN_SERVICE.to_string(),
            }
        }

        #[cfg(target_os = "windows")]
        {
            Self::WindowsCredentialManager {
                target_prefix: WINDOWS_CREDENTIAL_TARGET_PREFIX.to_string(),
            }
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            Self::LocalSqlite
        }
    }

    fn provider(&self) -> &'static str {
        match self {
            Self::LocalSqlite => LOCAL_SQLITE_SECRET_PROVIDER,
            Self::MacosKeychain { .. } => MACOS_KEYCHAIN_SECRET_PROVIDER,
            #[cfg(target_os = "windows")]
            Self::WindowsCredentialManager { .. } => WINDOWS_CREDENTIAL_MANAGER_SECRET_PROVIDER,
        }
    }

    fn secure_storage_label(&self) -> &'static str {
        match self {
            Self::LocalSqlite => "SQLite secret refs",
            Self::MacosKeychain { .. } => "macOS Keychain",
            #[cfg(target_os = "windows")]
            Self::WindowsCredentialManager { .. } => "Windows Credential Manager",
        }
    }

    fn keychain_service(&self) -> Option<&str> {
        match self {
            Self::LocalSqlite => None,
            Self::MacosKeychain { service } => Some(service),
            #[cfg(target_os = "windows")]
            Self::WindowsCredentialManager { .. } => None,
        }
    }

    #[cfg(target_os = "windows")]
    fn windows_credential_target(&self, secret_id: &str) -> Option<String> {
        match self {
            Self::WindowsCredentialManager { target_prefix } => {
                Some(format!("{target_prefix}:{secret_id}"))
            }
            _ => None,
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn windows_credential_target(&self, _secret_id: &str) -> Option<String> {
        None
    }
}

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
    secret_backend: SecretBackend,
    secret_migration_error: Arc<Mutex<Option<String>>>,
}

impl ProfileStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        Self::open_with_backend(path, SecretBackend::default_for_platform())
    }

    fn open_with_backend(
        path: impl AsRef<Path>,
        secret_backend: SecretBackend,
    ) -> Result<Self, StoreError> {
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
            secret_backend,
            secret_migration_error: Arc::new(Mutex::new(None)),
        };
        store.migrate()?;
        store.seed_defaults()?;
        store.try_migrate_legacy_stream_keys_to_backend();
        Ok(store)
    }

    pub fn open_memory() -> Result<Self, StoreError> {
        Self::open_memory_with_backend(SecretBackend::LocalSqlite)
    }

    fn open_memory_with_backend(secret_backend: SecretBackend) -> Result<Self, StoreError> {
        let connection = Connection::open_in_memory()?;
        let store = Self {
            connection: Arc::new(Mutex::new(connection)),
            secret_backend,
            secret_migration_error: Arc::new(Mutex::new(None)),
        };
        store.migrate()?;
        store.seed_defaults()?;
        store.try_migrate_legacy_stream_keys_to_backend();
        Ok(store)
    }

    pub fn profiles_snapshot(&self) -> Result<ProfilesSnapshot, StoreError> {
        Ok(ProfilesSnapshot {
            recording_profiles: self.list_recording_profiles()?,
            stream_destinations: self.list_stream_destinations()?,
        })
    }

    pub fn secret_storage_report(&self) -> Result<SecretStorageReport, StoreError> {
        let legacy_stream_key_refs = self.legacy_stream_key_ref_count()?;
        let migration_error = self
            .secret_migration_error
            .lock()
            .expect("secret migration mutex poisoned")
            .clone();

        let secret_storage_state = match (
            &self.secret_backend,
            migration_error.as_ref(),
            legacy_stream_key_refs,
        ) {
            (SecretBackend::MacosKeychain { .. }, Some(_), _) => "keychain-migration-failed",
            (SecretBackend::MacosKeychain { .. }, None, 0) => "keychain-ready",
            (SecretBackend::MacosKeychain { .. }, None, _) => "needs-keychain-migration",
            #[cfg(target_os = "windows")]
            (SecretBackend::WindowsCredentialManager { .. }, Some(_), _) => {
                "windows-credential-migration-failed"
            }
            #[cfg(target_os = "windows")]
            (SecretBackend::WindowsCredentialManager { .. }, None, 0) => {
                "windows-credential-manager-ready"
            }
            #[cfg(target_os = "windows")]
            (SecretBackend::WindowsCredentialManager { .. }, None, _) => {
                "needs-windows-credential-manager-migration"
            }
            (SecretBackend::LocalSqlite, _, _) => "local-sqlite-fallback",
        }
        .to_string();

        Ok(SecretStorageReport {
            secure_storage: self.secret_backend.secure_storage_label().to_string(),
            secret_storage_state,
            provider: self.secret_backend.provider().to_string(),
            legacy_stream_key_refs,
            migration_error,
        })
    }

    pub fn export_profile_bundle(&self) -> Result<ProfileBundle, StoreError> {
        let recording_profiles = self
            .list_recording_profiles()?
            .into_iter()
            .map(media_profile_input_from_model)
            .collect();
        let stream_destinations = self
            .list_stream_destinations()?
            .into_iter()
            .map(|destination| StreamDestinationBundleItem {
                name: destination.name,
                platform: destination.platform,
                ingest_url: destination.ingest_url,
                enabled: destination.enabled,
                has_stream_key: destination.stream_key_ref.is_some(),
            })
            .collect();

        Ok(ProfileBundle {
            version: 1,
            exported_at: now_utc(),
            recording_profiles,
            stream_destinations,
        })
    }

    pub fn import_profile_bundle(
        &self,
        bundle: ProfileBundle,
    ) -> Result<ProfileBundleImportResult, StoreError> {
        if bundle.version != 1 {
            return Err(StoreError::InvalidValue(format!(
                "unsupported profile bundle version {}",
                bundle.version
            )));
        }

        let mut recording_profiles = 0;
        for profile in bundle.recording_profiles {
            self.insert_recording_profile(profile)?;
            recording_profiles += 1;
        }

        let mut stream_destinations = 0;
        for destination in bundle.stream_destinations {
            self.insert_stream_destination(StreamDestinationInput {
                name: destination.name,
                platform: destination.platform,
                ingest_url: Some(destination.ingest_url),
                stream_key: None,
                enabled: Some(destination.enabled),
            })?;
            stream_destinations += 1;
        }

        Ok(ProfileBundleImportResult {
            recording_profiles,
            stream_destinations,
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

    pub fn scene_collection(&self) -> Result<SceneCollection, StoreError> {
        if let Some(collection) = self.read_scene_collection()? {
            return Ok(collection);
        }

        self.save_scene_collection(SceneCollection::default_collection(now_utc()))
    }

    pub fn export_scene_collection(&self) -> Result<SceneCollectionBundle, StoreError> {
        Ok(SceneCollectionBundle::new(
            self.scene_collection()?,
            now_utc(),
        ))
    }

    pub fn import_scene_collection(
        &self,
        bundle: SceneCollectionBundle,
    ) -> Result<SceneCollectionImportResult, StoreError> {
        if bundle.version != 1 {
            return Err(StoreError::InvalidValue(format!(
                "unsupported scene collection bundle version {}",
                bundle.version
            )));
        }

        let imported_scenes = bundle.collection.scenes.len();
        let imported_transitions = bundle.collection.transitions.len();
        let collection = self.save_scene_collection(bundle.collection)?;

        Ok(SceneCollectionImportResult {
            imported_scenes,
            imported_transitions,
            collection,
        })
    }

    pub fn save_scene_collection(
        &self,
        mut collection: SceneCollection,
    ) -> Result<SceneCollection, StoreError> {
        collection.id = collection.id.trim().to_string();
        collection.name = collection.name.trim().to_string();
        collection.updated_at = now_utc();
        validate_scene_collection_for_store(&collection)?;

        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        connection.execute(
            "INSERT INTO scene_collections (id, value_json, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET
               value_json = excluded.value_json,
               updated_at = excluded.updated_at",
            params![
                &collection.id,
                serde_json::to_string(&collection)?,
                collection.updated_at.to_rfc3339(),
            ],
        )?;

        Ok(collection)
    }

    pub fn validate_scene_collection(&self, collection: &SceneCollection) -> SceneValidationResult {
        collection.validation()
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

    pub fn create_marker(&self, input: MarkerCreateInput) -> Result<MarkerWriteResult, StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let MarkerCreateInput {
            label,
            source_app,
            source_event_id,
            recording_session_id,
            media_path,
            start_seconds,
            end_seconds,
            metadata,
        } = input;

        if let (Some(source_app), Some(source_event_id)) = (&source_app, &source_event_id) {
            if let Some(marker) = marker_by_source_event(&connection, source_app, source_event_id)?
            {
                return Ok(MarkerWriteResult {
                    marker,
                    created: false,
                });
            }
        }

        let marker = Marker {
            id: new_id("marker"),
            label,
            source_app,
            source_event_id,
            recording_session_id,
            media_path,
            start_seconds,
            end_seconds,
            metadata: metadata.unwrap_or_else(|| json!({})),
            created_at: now_utc(),
        };

        connection.execute(
            "INSERT INTO markers
             (id, label, source_app, source_event_id, recording_session_id, media_path, start_seconds, end_seconds, metadata_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                &marker.id,
                &marker.label,
                &marker.source_app,
                &marker.source_event_id,
                &marker.recording_session_id,
                &marker.media_path,
                marker.start_seconds,
                marker.end_seconds,
                serde_json::to_string(&marker.metadata)?,
                marker.created_at.to_rfc3339()
            ],
        )?;

        Ok(MarkerWriteResult {
            marker,
            created: true,
        })
    }

    pub fn list_markers(&self, filters: MarkerFilters) -> Result<Vec<Marker>, StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let limit = filters.limit.unwrap_or(100).clamp(1, MARKER_LIST_LIMIT);
        let mut statement = connection.prepare(
            "SELECT id, label, source_app, source_event_id, recording_session_id, media_path,
                    start_seconds, end_seconds, metadata_json, created_at
             FROM markers
             WHERE (?1 IS NULL OR source_app = ?1)
               AND (?2 IS NULL OR source_event_id = ?2)
               AND (?3 IS NULL OR recording_session_id = ?3)
             ORDER BY created_at DESC
             LIMIT ?4",
        )?;

        let rows = statement.query_map(
            params![
                filters.source_app,
                filters.source_event_id,
                filters.recording_session_id,
                limit as u32,
            ],
            marker_record_from_row,
        )?;

        rows.map(|row| marker_from_record(row?)).collect()
    }

    pub fn record_stopped_recording(
        &self,
        session: &RecordingSession,
    ) -> Result<RecordingHistoryEntry, StoreError> {
        let entry = RecordingHistoryEntry {
            session_id: session.id.clone(),
            output_path: session.output_path.clone(),
            profile_id: session.profile.id.clone(),
            profile_name: session.profile.name.clone(),
            started_at: session.started_at,
            stopped_at: now_utc(),
        };
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        connection.execute(
            "INSERT INTO recording_history
             (session_id, output_path, profile_id, profile_name, started_at, stopped_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(session_id) DO UPDATE SET
               output_path = excluded.output_path,
               profile_id = excluded.profile_id,
               profile_name = excluded.profile_name,
               started_at = excluded.started_at,
               stopped_at = excluded.stopped_at",
            params![
                entry.session_id,
                entry.output_path,
                entry.profile_id,
                entry.profile_name,
                entry.started_at.to_rfc3339(),
                entry.stopped_at.to_rfc3339()
            ],
        )?;

        Ok(entry)
    }

    pub fn list_recent_recordings(
        &self,
        limit: usize,
    ) -> Result<Vec<RecordingHistoryEntry>, StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let mut statement = connection.prepare(
            "SELECT session_id, output_path, profile_id, profile_name, started_at, stopped_at
             FROM recording_history
             ORDER BY stopped_at DESC
             LIMIT ?1",
        )?;

        let entries = statement
            .query_map([limit as i64], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })?
            .map(|entry| {
                let (session_id, output_path, profile_id, profile_name, started_at, stopped_at) =
                    entry?;
                Ok(RecordingHistoryEntry {
                    session_id,
                    output_path,
                    profile_id,
                    profile_name,
                    started_at: parse_time(&started_at)?,
                    stopped_at: parse_time(&stopped_at)?,
                })
            })
            .collect::<Result<Vec<_>, StoreError>>()?;

        Ok(entries)
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
        if current_version < 3 {
            apply_migration_3(&connection)?;
        }
        if current_version < 4 {
            apply_migration_4(&connection)?;
        }
        if current_version < 5 {
            apply_migration_5(&connection)?;
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

    fn read_scene_collection(&self) -> Result<Option<SceneCollection>, StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let value = connection
            .query_row(
                "SELECT value_json FROM scene_collections ORDER BY updated_at DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        value
            .map(|value| serde_json::from_str(&value).map_err(StoreError::Json))
            .transpose()
    }

    fn try_migrate_legacy_stream_keys_to_backend(&self) {
        if matches!(self.secret_backend, SecretBackend::LocalSqlite) {
            return;
        }

        match self.migrate_legacy_stream_keys_to_backend() {
            Ok(()) => self.set_secret_migration_error(None),
            Err(error) => self.set_secret_migration_error(Some(error.to_string())),
        }
    }

    fn migrate_legacy_stream_keys_to_backend(&self) -> Result<(), StoreError> {
        let legacy_secrets = {
            let connection = self
                .connection
                .lock()
                .expect("profile store mutex poisoned");
            let mut statement = connection.prepare(
                "SELECT DISTINCT d.stream_key_ref_id, s.secret
                 FROM stream_destinations d
                 JOIN secrets s ON s.id = d.stream_key_ref_id
                 WHERE d.stream_key_ref_provider = ?1
                   AND d.stream_key_ref_id IS NOT NULL",
            )?;
            let rows = statement.query_map([LOCAL_SQLITE_SECRET_PROVIDER], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        if legacy_secrets.is_empty() {
            return Ok(());
        }

        for (secret_id, secret) in &legacy_secrets {
            write_backend_secret(&self.secret_backend, secret_id, secret.as_bytes())
                .map_err(StoreError::SecretStore)?;
        }

        let mut connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let transaction = connection.transaction()?;
        for (secret_id, _) in &legacy_secrets {
            transaction.execute(
                "UPDATE stream_destinations
                 SET stream_key_ref_provider = ?1
                 WHERE stream_key_ref_provider = ?2
                   AND stream_key_ref_id = ?3",
                params![
                    self.secret_backend.provider(),
                    LOCAL_SQLITE_SECRET_PROVIDER,
                    secret_id,
                ],
            )?;
            transaction.execute("DELETE FROM secrets WHERE id = ?1", [secret_id])?;
        }
        transaction.commit()?;
        Ok(())
    }

    fn set_secret_migration_error(&self, error: Option<String>) {
        *self
            .secret_migration_error
            .lock()
            .expect("secret migration mutex poisoned") = error;
    }

    fn legacy_stream_key_ref_count(&self) -> Result<usize, StoreError> {
        let connection = self
            .connection
            .lock()
            .expect("profile store mutex poisoned");
        let count = connection.query_row(
            "SELECT COUNT(*)
             FROM stream_destinations
             WHERE stream_key_ref_provider = ?1
               AND stream_key_ref_id IS NOT NULL",
            [LOCAL_SQLITE_SECRET_PROVIDER],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(count as usize)
    }

    fn delete_secret_ref(&self, reference: Option<&SecretRef>) -> Result<(), StoreError> {
        let Some(reference) = reference else {
            return Ok(());
        };

        match reference.provider.as_str() {
            LOCAL_SQLITE_SECRET_PROVIDER => {
                let connection = self
                    .connection
                    .lock()
                    .expect("profile store mutex poisoned");
                connection.execute("DELETE FROM secrets WHERE id = ?1", [&reference.id])?;
            }
            MACOS_KEYCHAIN_SECRET_PROVIDER => {
                if let Some(service) = self.secret_backend.keychain_service() {
                    delete_keychain_secret(service, &reference.id)
                        .map_err(StoreError::SecretStore)?;
                }
            }
            WINDOWS_CREDENTIAL_MANAGER_SECRET_PROVIDER => {
                if let Some(target) = self.secret_backend.windows_credential_target(&reference.id) {
                    delete_windows_credential_secret(&target).map_err(StoreError::SecretStore)?;
                }
            }
            _ => {}
        }

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

fn apply_migration_3(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch(
        r#"
            ALTER TABLE markers ADD COLUMN source_app TEXT;
            ALTER TABLE markers ADD COLUMN source_event_id TEXT;
            ALTER TABLE markers ADD COLUMN recording_session_id TEXT;
            ALTER TABLE markers ADD COLUMN media_path TEXT;
            ALTER TABLE markers ADD COLUMN start_seconds REAL;
            ALTER TABLE markers ADD COLUMN end_seconds REAL;
            ALTER TABLE markers ADD COLUMN metadata_json TEXT NOT NULL DEFAULT '{}';

            CREATE TABLE IF NOT EXISTS recording_history (
              session_id TEXT PRIMARY KEY,
              output_path TEXT NOT NULL,
              profile_id TEXT NOT NULL,
              profile_name TEXT NOT NULL,
              started_at TEXT NOT NULL,
              stopped_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_recording_history_stopped_at
            ON recording_history(stopped_at DESC);
            "#,
    )?;
    connection.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
        params![3_u32, now_utc().to_rfc3339()],
    )?;
    Ok(())
}

fn apply_migration_4(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch(
        r#"
            CREATE INDEX IF NOT EXISTS idx_markers_source_event
            ON markers(source_app, source_event_id);

            CREATE INDEX IF NOT EXISTS idx_markers_recording_session
            ON markers(recording_session_id, created_at DESC);

            CREATE INDEX IF NOT EXISTS idx_markers_created_at
            ON markers(created_at DESC);
            "#,
    )?;
    connection.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
        params![4_u32, now_utc().to_rfc3339()],
    )?;
    Ok(())
}

fn apply_migration_5(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch(
        r#"
            CREATE TABLE IF NOT EXISTS scene_collections (
              id TEXT PRIMARY KEY,
              value_json TEXT NOT NULL,
              updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_scene_collections_updated_at
            ON scene_collections(updated_at DESC);
            "#,
    )?;
    connection.execute(
        "INSERT INTO schema_migrations (version, applied_at) VALUES (?1, ?2)",
        params![5_u32, now_utc().to_rfc3339()],
    )?;
    Ok(())
}

fn validate_scene_collection_for_store(collection: &SceneCollection) -> Result<(), StoreError> {
    let validation = collection.validation();
    if validation.ok {
        return Ok(());
    }

    let messages = validation
        .issues
        .into_iter()
        .map(|issue| format!("{}: {}", issue.path, issue.message))
        .collect::<Vec<_>>()
        .join("; ");
    Err(StoreError::InvalidValue(format!(
        "invalid scene collection: {messages}"
    )))
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

fn write_backend_secret(
    backend: &SecretBackend,
    secret_id: &str,
    secret: &[u8],
) -> Result<SecretRef, SecretStoreError> {
    match backend {
        SecretBackend::LocalSqlite => Err(SecretStoreError::Store(
            "local SQLite secret writes must use the ProfileStore connection".to_string(),
        )),
        SecretBackend::MacosKeychain { service } => {
            write_keychain_secret(service, secret_id, secret)?;
            Ok(SecretRef::macos_keychain(secret_id))
        }
        #[cfg(target_os = "windows")]
        SecretBackend::WindowsCredentialManager { .. } => {
            let Some(target) = backend.windows_credential_target(secret_id) else {
                return Err(SecretStoreError::Store(
                    "Windows credential target was unavailable".to_string(),
                ));
            };
            write_windows_credential_secret(&target, secret)?;
            Ok(SecretRef::windows_credential_manager(secret_id))
        }
    }
}

#[cfg(target_os = "macos")]
fn write_keychain_secret(
    service: &str,
    account: &str,
    secret: &[u8],
) -> Result<(), SecretStoreError> {
    security_framework::passwords::set_generic_password(service, account, secret)
        .map_err(|error| SecretStoreError::Store(format!("macOS Keychain write failed: {error}")))
}

#[cfg(not(target_os = "macos"))]
fn write_keychain_secret(
    _service: &str,
    _account: &str,
    _secret: &[u8],
) -> Result<(), SecretStoreError> {
    Err(SecretStoreError::Store(
        "macOS Keychain is unavailable on this platform".to_string(),
    ))
}

#[cfg(target_os = "macos")]
fn read_keychain_secret(
    service: &str,
    account: &str,
) -> Result<Option<SensitiveString>, SecretStoreError> {
    match security_framework::passwords::get_generic_password(service, account) {
        Ok(bytes) => {
            let value = String::from_utf8(bytes).map_err(|error| {
                SecretStoreError::Store(format!(
                    "macOS Keychain value was not valid UTF-8: {error}"
                ))
            })?;
            Ok(Some(SensitiveString::new(value)))
        }
        Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(None),
        Err(error) => Err(SecretStoreError::Store(format!(
            "macOS Keychain read failed: {error}"
        ))),
    }
}

#[cfg(not(target_os = "macos"))]
fn read_keychain_secret(
    _service: &str,
    _account: &str,
) -> Result<Option<SensitiveString>, SecretStoreError> {
    Err(SecretStoreError::Store(
        "macOS Keychain is unavailable on this platform".to_string(),
    ))
}

#[cfg(target_os = "macos")]
fn delete_keychain_secret(service: &str, account: &str) -> Result<(), SecretStoreError> {
    match security_framework::passwords::delete_generic_password(service, account) {
        Ok(()) => Ok(()),
        Err(error) if error.code() == ERR_SEC_ITEM_NOT_FOUND => Ok(()),
        Err(error) => Err(SecretStoreError::Store(format!(
            "macOS Keychain delete failed: {error}"
        ))),
    }
}

#[cfg(not(target_os = "macos"))]
fn delete_keychain_secret(_service: &str, _account: &str) -> Result<(), SecretStoreError> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn write_windows_credential_secret(target: &str, secret: &[u8]) -> Result<(), SecretStoreError> {
    use std::ptr;
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::Security::Credentials::{
        CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE, CRED_TYPE_GENERIC,
    };

    let mut target_name = windows_wide(target);
    let mut user_name = windows_wide("vaexcore studio");
    let mut secret_bytes = secret.to_vec();
    let credential = CREDENTIALW {
        Flags: 0,
        Type: CRED_TYPE_GENERIC,
        TargetName: target_name.as_mut_ptr(),
        Comment: ptr::null_mut(),
        LastWritten: FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        },
        CredentialBlobSize: secret_bytes.len() as u32,
        CredentialBlob: secret_bytes.as_mut_ptr(),
        Persist: CRED_PERSIST_LOCAL_MACHINE,
        AttributeCount: 0,
        Attributes: ptr::null_mut(),
        TargetAlias: ptr::null_mut(),
        UserName: user_name.as_mut_ptr(),
    };

    let ok = unsafe { CredWriteW(&credential, 0) };
    if ok == 0 {
        return Err(SecretStoreError::Store(format!(
            "Windows Credential Manager write failed: {}",
            windows_last_error_message()
        )));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn read_windows_credential_secret(
    target: &str,
) -> Result<Option<SensitiveString>, SecretStoreError> {
    use std::{ptr, slice};
    use windows_sys::Win32::Foundation::ERROR_NOT_FOUND;
    use windows_sys::Win32::Security::Credentials::{
        CredFree, CredReadW, CREDENTIALW, CRED_TYPE_GENERIC,
    };

    let target_name = windows_wide(target);
    let mut credential: *mut CREDENTIALW = ptr::null_mut();
    let ok = unsafe { CredReadW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0, &mut credential) };
    if ok == 0 {
        let code = windows_last_error_code();
        if code == ERROR_NOT_FOUND {
            return Ok(None);
        }
        return Err(SecretStoreError::Store(format!(
            "Windows Credential Manager read failed: {}",
            windows_last_error_message()
        )));
    }

    let credential_ref = unsafe { &*credential };
    let bytes = unsafe {
        slice::from_raw_parts(
            credential_ref.CredentialBlob,
            credential_ref.CredentialBlobSize as usize,
        )
    };
    let value_bytes = bytes.to_vec();
    unsafe { CredFree(credential.cast()) };
    let value = String::from_utf8(value_bytes).map_err(|error| {
        SecretStoreError::Store(format!(
            "Windows Credential Manager value was not valid UTF-8: {error}"
        ))
    })?;
    Ok(Some(SensitiveString::new(value)))
}

#[cfg(not(target_os = "windows"))]
fn read_windows_credential_secret(
    _target: &str,
) -> Result<Option<SensitiveString>, SecretStoreError> {
    Err(SecretStoreError::Store(
        "Windows Credential Manager is unavailable on this platform".to_string(),
    ))
}

#[cfg(target_os = "windows")]
fn delete_windows_credential_secret(target: &str) -> Result<(), SecretStoreError> {
    use windows_sys::Win32::Foundation::ERROR_NOT_FOUND;
    use windows_sys::Win32::Security::Credentials::{CredDeleteW, CRED_TYPE_GENERIC};

    let target_name = windows_wide(target);
    let ok = unsafe { CredDeleteW(target_name.as_ptr(), CRED_TYPE_GENERIC, 0) };
    if ok == 0 {
        let code = windows_last_error_code();
        if code == ERROR_NOT_FOUND {
            return Ok(());
        }
        return Err(SecretStoreError::Store(format!(
            "Windows Credential Manager delete failed: {}",
            windows_last_error_message()
        )));
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn delete_windows_credential_secret(_target: &str) -> Result<(), SecretStoreError> {
    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_wide(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(target_os = "windows")]
fn windows_last_error_code() -> u32 {
    unsafe { windows_sys::Win32::Foundation::GetLastError() }
}

#[cfg(target_os = "windows")]
fn windows_last_error_message() -> String {
    let code = windows_last_error_code();
    format!("Windows error {code}")
}

impl SecretStore for ProfileStore {
    fn put_secret(
        &self,
        scope: &str,
        value: &SensitiveString,
    ) -> Result<SecretRef, SecretStoreError> {
        let id = new_id("secret");

        match &self.secret_backend {
            SecretBackend::LocalSqlite => {
                let connection = self.connection.lock().map_err(|_| {
                    SecretStoreError::Store("profile store mutex poisoned".to_string())
                })?;

                connection
                    .execute(
                        "INSERT INTO secrets (id, scope, secret, created_at) VALUES (?1, ?2, ?3, ?4)",
                        params![id, scope, value.expose_secret(), now_utc().to_rfc3339()],
                    )
                    .map_err(|error| SecretStoreError::Store(error.to_string()))?;

                Ok(SecretRef::local(id))
            }
            SecretBackend::MacosKeychain { service } => {
                write_keychain_secret(service, &id, value.expose_secret().as_bytes())?;
                Ok(SecretRef::macos_keychain(id))
            }
            #[cfg(target_os = "windows")]
            SecretBackend::WindowsCredentialManager { .. } => {
                let Some(target) = self.secret_backend.windows_credential_target(&id) else {
                    return Err(SecretStoreError::Store(
                        "Windows credential target was unavailable".to_string(),
                    ));
                };
                write_windows_credential_secret(&target, value.expose_secret().as_bytes())?;
                Ok(SecretRef::windows_credential_manager(id))
            }
        }
    }

    fn get_secret(
        &self,
        reference: &SecretRef,
    ) -> Result<Option<SensitiveString>, SecretStoreError> {
        match reference.provider.as_str() {
            LOCAL_SQLITE_SECRET_PROVIDER => {
                let connection = self.connection.lock().map_err(|_| {
                    SecretStoreError::Store("profile store mutex poisoned".to_string())
                })?;

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
            MACOS_KEYCHAIN_SECRET_PROVIDER => {
                if let Some(service) = self.secret_backend.keychain_service() {
                    read_keychain_secret(service, &reference.id)
                } else {
                    Ok(None)
                }
            }
            WINDOWS_CREDENTIAL_MANAGER_SECRET_PROVIDER => {
                if let Some(target) = self.secret_backend.windows_credential_target(&reference.id) {
                    read_windows_credential_secret(&target)
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
}

fn media_profile_input_from_model(profile: MediaProfile) -> MediaProfileInput {
    MediaProfileInput {
        name: profile.name,
        output_folder: profile.output_folder,
        filename_pattern: profile.filename_pattern,
        container: profile.container,
        resolution: profile.resolution,
        framerate: profile.framerate,
        bitrate_kbps: profile.bitrate_kbps,
        encoder_preference: profile.encoder_preference,
    }
}

type MarkerRecord = (
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<f64>,
    Option<f64>,
    String,
    String,
);

fn marker_record_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MarkerRecord> {
    Ok((
        row.get::<_, String>(0)?,
        row.get::<_, Option<String>>(1)?,
        row.get::<_, Option<String>>(2)?,
        row.get::<_, Option<String>>(3)?,
        row.get::<_, Option<String>>(4)?,
        row.get::<_, Option<String>>(5)?,
        row.get::<_, Option<f64>>(6)?,
        row.get::<_, Option<f64>>(7)?,
        row.get::<_, String>(8)?,
        row.get::<_, String>(9)?,
    ))
}

fn marker_from_record(record: MarkerRecord) -> Result<Marker, StoreError> {
    let (
        id,
        label,
        source_app,
        source_event_id,
        recording_session_id,
        media_path,
        start_seconds,
        end_seconds,
        metadata_json,
        created_at,
    ) = record;

    Ok(Marker {
        id,
        label,
        source_app,
        source_event_id,
        recording_session_id,
        media_path,
        start_seconds,
        end_seconds,
        metadata: serde_json::from_str(&metadata_json)?,
        created_at: parse_time(&created_at)?,
    })
}

fn marker_by_source_event(
    connection: &Connection,
    source_app: &str,
    source_event_id: &str,
) -> Result<Option<Marker>, StoreError> {
    let record = connection
        .query_row(
            "SELECT id, label, source_app, source_event_id, recording_session_id, media_path,
                    start_seconds, end_seconds, metadata_json, created_at
             FROM markers
             WHERE source_app = ?1 AND source_event_id = ?2
             ORDER BY created_at DESC
             LIMIT 1",
            params![source_app, source_event_id],
            marker_record_from_row,
        )
        .optional()?;

    record.map(marker_from_record).transpose()
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
    fn scene_collection_round_trip() {
        let store = ProfileStore::open_memory().unwrap();
        let mut collection = store.scene_collection().unwrap();
        assert_eq!(collection.active_scene_id, "scene-main");

        collection.name = "Updated Scenes".to_string();
        collection.scenes[0].name = "Updated Main".to_string();
        let saved = store.save_scene_collection(collection.clone()).unwrap();

        assert_eq!(saved.name, "Updated Scenes");
        assert_eq!(
            store.scene_collection().unwrap().scenes[0].name,
            "Updated Main"
        );
    }

    #[test]
    fn invalid_scene_collection_is_rejected() {
        let store = ProfileStore::open_memory().unwrap();
        let mut collection = store.scene_collection().unwrap();
        collection.active_scene_id = "missing".to_string();

        let error = store.save_scene_collection(collection).unwrap_err();

        assert!(error
            .to_string()
            .contains("Active scene id must match a scene"));
    }

    #[test]
    fn scene_collection_bundle_round_trip_replaces_collection() {
        let source = ProfileStore::open_memory().unwrap();
        let mut bundle = source.export_scene_collection().unwrap();
        bundle.collection.name = "Imported Scene Collection".to_string();
        bundle.collection.scenes[0].name = "Imported Main".to_string();

        let target = ProfileStore::open_memory().unwrap();
        let result = target.import_scene_collection(bundle).unwrap();

        assert_eq!(result.imported_scenes, 1);
        assert_eq!(result.imported_transitions, 2);
        assert_eq!(result.collection.name, "Imported Scene Collection");
        assert_eq!(
            target.scene_collection().unwrap().scenes[0].name,
            "Imported Main"
        );
    }

    #[test]
    fn stream_destination_secret_round_trip_uses_configured_store() {
        let store = ProfileStore::open_memory().unwrap();
        let destination = store
            .insert_stream_destination(StreamDestinationInput {
                name: "Secret RTMP".to_string(),
                platform: PlatformKind::CustomRtmp,
                ingest_url: Some("rtmps://example.test/live".to_string()),
                stream_key: Some(SensitiveString::new("local-secret-key")),
                enabled: Some(true),
            })
            .unwrap();
        let reference = destination.stream_key_ref.clone().unwrap();

        assert_eq!(reference.provider, LOCAL_SQLITE_SECRET_PROVIDER);
        assert_eq!(
            store
                .get_secret(&reference)
                .unwrap()
                .map(|secret| secret.expose_secret().to_string()),
            Some("local-secret-key".to_string())
        );

        assert!(store.delete_stream_destination(&destination.id).unwrap());
        assert!(store.get_secret(&reference).unwrap().is_none());

        let report = store.secret_storage_report().unwrap();
        assert_eq!(report.secret_storage_state, "local-sqlite-fallback");
        assert_eq!(report.legacy_stream_key_refs, 0);
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore = "writes a temporary generic password to the login Keychain"]
    fn macos_keychain_secret_round_trip() {
        let service = format!("com.vaexcore.studio.tests.{}", new_id("service"));
        let store = ProfileStore::open_memory_with_backend(SecretBackend::MacosKeychain {
            service: service.clone(),
        })
        .unwrap();
        let destination = store
            .insert_stream_destination(StreamDestinationInput {
                name: "Keychain RTMP".to_string(),
                platform: PlatformKind::CustomRtmp,
                ingest_url: Some("rtmps://example.test/live".to_string()),
                stream_key: Some(SensitiveString::new("keychain-secret-key")),
                enabled: Some(true),
            })
            .unwrap();
        let reference = destination.stream_key_ref.clone().unwrap();

        assert_eq!(reference.provider, MACOS_KEYCHAIN_SECRET_PROVIDER);
        assert_eq!(
            store
                .get_secret(&reference)
                .unwrap()
                .map(|secret| secret.expose_secret().to_string()),
            Some("keychain-secret-key".to_string())
        );

        assert!(store.delete_stream_destination(&destination.id).unwrap());
        assert!(store.get_secret(&reference).unwrap().is_none());
        let _ = delete_keychain_secret(&service, &reference.id);
    }

    #[test]
    #[cfg(target_os = "macos")]
    #[ignore = "writes a temporary generic password to the login Keychain"]
    fn macos_keychain_migrates_legacy_sqlite_stream_keys() {
        let temp_dir = tempfile::tempdir().unwrap();
        let database_path = temp_dir.path().join("profiles.sqlite3");
        let service = format!("com.vaexcore.studio.tests.{}", new_id("service"));

        let legacy_reference = {
            let store = ProfileStore::open_with_backend(&database_path, SecretBackend::LocalSqlite)
                .unwrap();
            let destination = store
                .insert_stream_destination(StreamDestinationInput {
                    name: "Legacy RTMP".to_string(),
                    platform: PlatformKind::CustomRtmp,
                    ingest_url: Some("rtmps://example.test/live".to_string()),
                    stream_key: Some(SensitiveString::new("legacy-secret-key")),
                    enabled: Some(true),
                })
                .unwrap();
            destination.stream_key_ref.unwrap()
        };
        assert_eq!(legacy_reference.provider, LOCAL_SQLITE_SECRET_PROVIDER);

        let store = ProfileStore::open_with_backend(
            &database_path,
            SecretBackend::MacosKeychain {
                service: service.clone(),
            },
        )
        .unwrap();
        let migrated = store
            .list_stream_destinations()
            .unwrap()
            .into_iter()
            .find(|destination| destination.name == "Legacy RTMP")
            .unwrap();
        let migrated_reference = migrated.stream_key_ref.unwrap();

        assert_eq!(migrated_reference.id, legacy_reference.id);
        assert_eq!(migrated_reference.provider, MACOS_KEYCHAIN_SECRET_PROVIDER);
        assert_eq!(
            store
                .get_secret(&migrated_reference)
                .unwrap()
                .map(|secret| secret.expose_secret().to_string()),
            Some("legacy-secret-key".to_string())
        );

        let report = store.secret_storage_report().unwrap();
        assert_eq!(report.secret_storage_state, "keychain-ready");
        assert_eq!(report.legacy_stream_key_refs, 0);

        let connection = store.connection.lock().unwrap();
        let stored_secret_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM secrets WHERE id = ?1",
                [&migrated_reference.id],
                |row| row.get(0),
            )
            .unwrap();
        drop(connection);
        assert_eq!(stored_secret_count, 0);

        let _ = delete_keychain_secret(&service, &migrated_reference.id);
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

    #[test]
    fn marker_round_trip_accepts_connected_app_fields() {
        let store = ProfileStore::open_memory().unwrap();

        let result = store
            .create_marker(MarkerCreateInput {
                label: Some("Pulse keep: opener".to_string()),
                source_app: Some("vaexcore-pulse".to_string()),
                source_event_id: Some("pulse:session:candidate".to_string()),
                recording_session_id: Some("rec_123".to_string()),
                media_path: Some("/tmp/recording.mkv".to_string()),
                start_seconds: Some(12.5),
                end_seconds: Some(24.0),
                metadata: Some(serde_json::json!({ "confidenceBand": "high" })),
            })
            .unwrap();
        let marker = result.marker;

        assert!(result.created);
        assert_eq!(marker.label.as_deref(), Some("Pulse keep: opener"));
        assert_eq!(marker.source_app.as_deref(), Some("vaexcore-pulse"));
        assert_eq!(marker.recording_session_id.as_deref(), Some("rec_123"));
        assert_eq!(marker.start_seconds, Some(12.5));
        assert_eq!(marker.end_seconds, Some(24.0));
        assert_eq!(marker.metadata["confidenceBand"], "high");

        let duplicate = store
            .create_marker(MarkerCreateInput {
                label: Some("Pulse keep: duplicate".to_string()),
                source_app: Some("vaexcore-pulse".to_string()),
                source_event_id: Some("pulse:session:candidate".to_string()),
                recording_session_id: Some("rec_123".to_string()),
                media_path: Some("/tmp/recording.mkv".to_string()),
                start_seconds: Some(12.5),
                end_seconds: Some(24.0),
                metadata: Some(serde_json::json!({ "confidenceBand": "low" })),
            })
            .unwrap();

        assert!(!duplicate.created);
        assert_eq!(duplicate.marker.id, marker.id);
        assert_eq!(
            duplicate.marker.label.as_deref(),
            Some("Pulse keep: opener")
        );
    }

    #[test]
    fn marker_list_filters_by_source_and_recording() {
        let store = ProfileStore::open_memory().unwrap();

        let pulse_marker = store
            .create_marker(MarkerCreateInput {
                label: Some("Pulse keep".to_string()),
                source_app: Some("vaexcore-pulse".to_string()),
                source_event_id: Some("pulse:one".to_string()),
                recording_session_id: Some("rec_123".to_string()),
                media_path: Some("/tmp/recording.mkv".to_string()),
                start_seconds: Some(1.0),
                end_seconds: Some(2.0),
                metadata: None,
            })
            .unwrap()
            .marker;
        store
            .create_marker(MarkerCreateInput {
                label: Some("Console marker".to_string()),
                source_app: Some("vaexcore-console".to_string()),
                source_event_id: Some("chat:one".to_string()),
                recording_session_id: Some("rec_123".to_string()),
                media_path: None,
                start_seconds: None,
                end_seconds: None,
                metadata: None,
            })
            .unwrap();

        let pulse_markers = store
            .list_markers(MarkerFilters {
                source_app: Some("vaexcore-pulse".to_string()),
                ..MarkerFilters::default()
            })
            .unwrap();
        assert_eq!(pulse_markers, vec![pulse_marker]);

        let recording_markers = store
            .list_markers(MarkerFilters {
                recording_session_id: Some("rec_123".to_string()),
                limit: Some(10),
                ..MarkerFilters::default()
            })
            .unwrap();
        assert_eq!(recording_markers.len(), 2);
    }

    #[test]
    fn recording_history_lists_newest_recordings_first() {
        let store = ProfileStore::open_memory().unwrap();
        let profile = store.recording_profile_by_id(None).unwrap().unwrap();
        let first = RecordingSession {
            id: "rec_first".to_string(),
            profile: profile.clone(),
            output_path: "/tmp/first.mkv".to_string(),
            started_at: now_utc(),
        };
        let second = RecordingSession {
            id: "rec_second".to_string(),
            profile,
            output_path: "/tmp/second.mkv".to_string(),
            started_at: now_utc(),
        };

        store.record_stopped_recording(&first).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
        store.record_stopped_recording(&second).unwrap();

        let recordings = store.list_recent_recordings(2).unwrap();
        assert_eq!(recordings.len(), 2);
        assert_eq!(recordings[0].session_id, "rec_second");
        assert_eq!(recordings[0].output_path, "/tmp/second.mkv");
        assert_eq!(recordings[1].session_id, "rec_first");
    }

    #[test]
    fn profile_bundle_exports_without_secret_values() {
        let store = ProfileStore::open_memory().unwrap();
        store
            .insert_stream_destination(StreamDestinationInput {
                name: "Secret RTMP".to_string(),
                platform: PlatformKind::CustomRtmp,
                ingest_url: Some("rtmps://example.test/live".to_string()),
                stream_key: Some(SensitiveString::new("super-secret-key")),
                enabled: Some(true),
            })
            .unwrap();

        let bundle = store.export_profile_bundle().unwrap();
        let json = serde_json::to_string(&bundle).unwrap();

        assert_eq!(bundle.version, 1);
        assert!(bundle
            .stream_destinations
            .iter()
            .any(|destination| destination.name == "Secret RTMP" && destination.has_stream_key));
        assert!(!json.contains("super-secret-key"));
    }

    #[test]
    fn profile_bundle_import_creates_new_profiles_without_secrets() {
        let source = ProfileStore::open_memory().unwrap();
        let bundle = source.export_profile_bundle().unwrap();
        let target = ProfileStore::open_memory().unwrap();

        let result = target.import_profile_bundle(bundle).unwrap();

        assert!(result.recording_profiles > 0);
        assert!(result.stream_destinations > 0);
        assert!(target
            .list_stream_destinations()
            .unwrap()
            .into_iter()
            .any(|destination| destination.stream_key_ref.is_none()));
    }
}
