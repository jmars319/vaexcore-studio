import {
  Activity,
  Cable,
  CheckCircle2,
  Copy,
  FileVideo,
  MapPin,
  Pencil,
  Play,
  Plus,
  Radio,
  ScrollText,
  Settings as SettingsIcon,
  SlidersHorizontal,
  Square,
  Terminal,
  Trash2,
  Video,
  WifiOff,
  X,
} from "lucide-react";
import { FormEvent, ReactNode, useEffect, useMemo, useState } from "react";
import type {
  AppSettings,
  AuditLogEntry,
  CaptureSourceCandidate,
  CaptureSourceInventory,
  CaptureSourceKind,
  CaptureSourceSelection,
  ConnectedClient,
  HealthResponse,
  MediaPipelinePlan,
  MediaProfile,
  MediaProfileInput,
  PlatformKind,
  PreflightSnapshot,
  PreflightStatus,
  ProfilesSnapshot,
  RecordingContainer,
  StudioEvent,
  StudioStatus,
  StreamDestination,
  StreamDestinationInput,
} from "@vaexcore/shared-types";
import { platformLabels } from "@vaexcore/shared-types";
import {
  eventSocketUrl,
  exportProfileBundle,
  importProfileBundle,
  LocalAppSettingsSnapshot,
  loadCameraPermissionStatus,
  loadCaptureSourceInventory,
  loadMediaRunnerInfo,
  loadMicrophonePermissionStatus,
  loadPreflightSnapshot,
  loadRuntimeConfig,
  loadAppSettings,
  MediaRunnerInfo,
  openDataDirectory,
  openCameraPrivacySettings,
  openMicrophonePrivacySettings,
  openScreenRecordingPrivacySettings,
  PermissionStatus,
  regenerateApiToken,
  RuntimeApiConfig,
  saveAppSettings,
  StudioApi,
} from "./api";
import logoUrl from "./assets/brand/vaexcore-studio-logo.jpg";

type Section =
  | "dashboard"
  | "destinations"
  | "profiles"
  | "controls"
  | "apps"
  | "logs";

const sectionIds: readonly Section[] = [
  "dashboard",
  "destinations",
  "profiles",
  "controls",
  "apps",
  "logs",
];

const navItems: Array<{ id: Section; label: string; icon: ReactNode }> = [
  { id: "dashboard", label: "Dashboard", icon: <Activity size={17} /> },
  { id: "destinations", label: "Stream Destinations", icon: <Radio size={17} /> },
  { id: "profiles", label: "Recording Profiles", icon: <FileVideo size={17} /> },
  { id: "controls", label: "Controls", icon: <SlidersHorizontal size={17} /> },
  { id: "apps", label: "Connected Apps", icon: <Cable size={17} /> },
  { id: "logs", label: "Logs", icon: <ScrollText size={17} /> },
];

const openSectionEvent = "vaexcore://open-section";

const defaultProfileForm: MediaProfileInput = {
  name: "1080p60 Local",
  output_folder: "~/Movies/vaexcore studio",
  filename_pattern: "{date}-{time}-{profile}",
  container: "mkv",
  resolution: { width: 1920, height: 1080 },
  framerate: 60,
  bitrate_kbps: 12000,
  encoder_preference: "auto",
};

const defaultDestinationForm: StreamDestinationInput = {
  name: "Twitch Primary",
  platform: "twitch",
  ingest_url: "rtmp://live.twitch.tv/app",
  stream_key: "",
  enabled: true,
};

const defaultAppSettings: AppSettings = {
  api_host: "127.0.0.1",
  api_port: 51287,
  api_token: null,
  dev_auth_bypass: true,
  log_level: "info",
  default_recording_profile: defaultProfileForm,
  capture_sources: [
    {
      id: "display:main",
      kind: "display",
      name: "Main Display",
      enabled: true,
    },
    {
      id: "microphone:default",
      kind: "microphone",
      name: "Default Microphone",
      enabled: false,
    },
  ],
};

function hostFromUrl(url: string, fallback: string): string {
  try {
    return new URL(url).host;
  } catch {
    return fallback;
  }
}

function App() {
  const isSettingsWindow = useMemo(
    () => new URLSearchParams(window.location.search).get("window") === "settings",
    [],
  );
  const [section, setSection] = useState<Section>("dashboard");
  const [config, setConfig] = useState<RuntimeApiConfig | null>(null);
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [status, setStatus] = useState<StudioStatus | null>(null);
  const [profiles, setProfiles] = useState<ProfilesSnapshot | null>(null);
  const [events, setEvents] = useState<StudioEvent[]>([]);
  const [clients, setClients] = useState<ConnectedClient[]>([]);
  const [auditEntries, setAuditEntries] = useState<AuditLogEntry[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedProfileId, setSelectedProfileId] = useState<string | undefined>();
  const [selectedDestinationId, setSelectedDestinationId] = useState<string | undefined>();
  const [profileForm, setProfileForm] = useState<MediaProfileInput>(defaultProfileForm);
  const [destinationForm, setDestinationForm] =
    useState<StreamDestinationInput>(defaultDestinationForm);
  const [editingProfileId, setEditingProfileId] = useState<string | null>(null);
  const [editingDestinationId, setEditingDestinationId] = useState<string | null>(null);
  const [markerLabel, setMarkerLabel] = useState("manual-marker");
  const [settingsSnapshot, setSettingsSnapshot] =
    useState<LocalAppSettingsSnapshot | null>(null);
  const [settingsForm, setSettingsForm] =
    useState<AppSettings>(defaultAppSettings);
  const [settingsMessage, setSettingsMessage] = useState<string | null>(null);
  const [mediaRunnerInfo, setMediaRunnerInfo] =
    useState<MediaRunnerInfo | null>(null);
  const [preflight, setPreflight] = useState<PreflightSnapshot | null>(null);
  const [captureInventory, setCaptureInventory] =
    useState<CaptureSourceInventory | null>(null);
  const [permissionStatuses, setPermissionStatuses] = useState<{
    camera: PermissionStatus | null;
    microphone: PermissionStatus | null;
  }>({ camera: null, microphone: null });
  const [pipelinePlan, setPipelinePlan] = useState<MediaPipelinePlan | null>(null);

  useEffect(() => {
    loadRuntimeConfig().then(setConfig).catch((error: Error) => {
      setError(error.message);
    });
  }, []);

  useEffect(() => {
    loadAppSettings()
      .then((snapshot) => {
        applySettingsSnapshot(snapshot);
        setProfileForm(snapshot.settings.default_recording_profile);
      })
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    refreshCaptureContext().catch(() => undefined);
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    import("@tauri-apps/api/event")
      .then(({ listen }) =>
        listen<string>(openSectionEvent, ({ payload }) => {
          if (isSection(payload)) {
            setSection(payload);
          }
        }),
      )
      .then((nextUnlisten) => {
        if (cancelled) {
          nextUnlisten();
        } else {
          unlisten = nextUnlisten;
        }
      })
      .catch(() => undefined);

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!config) return;
    const runtimeConfig = config;

    let cancelled = false;

    async function refresh() {
      try {
        const [
          nextHealth,
          nextStatus,
          nextProfiles,
          nextClients,
          nextAuditLog,
          nextMediaRunnerInfo,
          nextPipelinePlan,
        ] = await Promise.all([
          StudioApi.health(runtimeConfig),
          StudioApi.status(runtimeConfig),
          StudioApi.profiles(runtimeConfig),
          StudioApi.clients(runtimeConfig),
          StudioApi.auditLog(runtimeConfig),
          loadMediaRunnerInfo(),
          StudioApi.mediaPlan(runtimeConfig),
        ]);
        if (cancelled) return;
        setHealth(nextHealth);
        setStatus(nextStatus);
        setProfiles(nextProfiles);
        setClients(nextClients.clients);
        setAuditEntries(nextAuditLog.entries);
        setMediaRunnerInfo(nextMediaRunnerInfo);
        setPipelinePlan(nextPipelinePlan);
        loadPreflightSnapshot()
          .then((snapshot) => {
            if (!cancelled) setPreflight(snapshot);
          })
          .catch(() => undefined);
        setEvents((current) =>
          mergeEvents([...nextStatus.recent_events, ...current]),
        );
        setError(null);
        setSelectedProfileId((current) =>
          current &&
          nextProfiles.recording_profiles.some((profile) => profile.id === current)
            ? current
            : nextProfiles.recording_profiles[0]?.id,
        );
        setSelectedDestinationId((current) =>
          current &&
          nextProfiles.stream_destinations.some(
            (destination) => destination.id === current,
          )
            ? current
            : nextProfiles.stream_destinations[0]?.id,
        );
      } catch (error) {
        if (!cancelled) {
          setError(error instanceof Error ? error.message : "API unavailable");
        }
      }
    }

    refresh();
    const interval = window.setInterval(refresh, 2500);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, [config]);

  useEffect(() => {
    if (!config) return;

    const socket = new WebSocket(eventSocketUrl(config));
    socket.onmessage = (message) => {
      const event = JSON.parse(message.data) as StudioEvent;
      setEvents((current) => mergeEvents([event, ...current]));
      if (
        event.type === "recording.started" ||
        event.type === "recording.stopped" ||
        event.type === "stream.started" ||
        event.type === "stream.stopped"
      ) {
        StudioApi.status(config).then(setStatus).catch(() => undefined);
      }
    };
    socket.onerror = () => setError("WebSocket event stream unavailable");

    return () => socket.close();
  }, [config]);

  const activeStatus = status?.status;
  const activeDestination = activeStatus?.active_destination;
  const recordingPath = activeStatus?.recording_path;

  async function refreshProfiles() {
    if (!config) return;
    const nextProfiles = await StudioApi.profiles(config);
    setProfiles(nextProfiles);
    setSelectedProfileId((current) =>
      current &&
      nextProfiles.recording_profiles.some((profile) => profile.id === current)
        ? current
        : nextProfiles.recording_profiles[0]?.id,
    );
    setSelectedDestinationId((current) =>
      current &&
      nextProfiles.stream_destinations.some(
        (destination) => destination.id === current,
      )
        ? current
        : nextProfiles.stream_destinations[0]?.id,
    );
    setEditingProfileId((current) =>
      current &&
      nextProfiles.recording_profiles.some((profile) => profile.id === current)
        ? current
        : null,
    );
    setEditingDestinationId((current) =>
      current &&
      nextProfiles.stream_destinations.some(
        (destination) => destination.id === current,
      )
        ? current
        : null,
    );
  }

  async function refreshCaptureContext() {
    const [inventory, camera, microphone] = await Promise.all([
      loadCaptureSourceInventory(),
      loadCameraPermissionStatus(),
      loadMicrophonePermissionStatus(),
    ]);
    setCaptureInventory(inventory);
    setPermissionStatuses({ camera, microphone });
  }

  async function runCommand(action: () => Promise<{ status: StudioStatus["status"] }>) {
    try {
      const result = await action();
      setStatus((current) => ({
        recent_events: current?.recent_events ?? [],
        status: result.status,
      }));
      setError(null);
    } catch (error) {
      setError(error instanceof Error ? error.message : "Command failed");
    }
  }

  async function createRecordingProfile(event: FormEvent) {
    event.preventDefault();
    if (!config) return;
    try {
      if (editingProfileId) {
        await StudioApi.updateRecordingProfile(config, editingProfileId, profileForm);
      } else {
        await StudioApi.createProfile(config, {
          kind: "recording_profile",
          value: profileForm,
        });
      }
      await refreshProfiles();
      setEditingProfileId(null);
      setError(null);
    } catch (error) {
      setError(error instanceof Error ? error.message : "Profile save failed");
    }
  }

  async function createDestination(event: FormEvent) {
    event.preventDefault();
    if (!config) return;
    try {
      const value = {
        ...destinationForm,
        stream_key: destinationForm.stream_key || null,
      };
      if (editingDestinationId) {
        await StudioApi.updateStreamDestination(config, editingDestinationId, value);
      } else {
        await StudioApi.createProfile(config, {
          kind: "stream_destination",
          value,
        });
      }
      await refreshProfiles();
      setEditingDestinationId(null);
      setError(null);
    } catch (error) {
      setError(
        error instanceof Error ? error.message : "Destination save failed",
      );
    }
  }

  function editRecordingProfile(profile: MediaProfile) {
    setEditingProfileId(profile.id);
    setProfileForm({
      name: profile.name,
      output_folder: profile.output_folder,
      filename_pattern: profile.filename_pattern,
      container: profile.container,
      resolution: profile.resolution,
      framerate: profile.framerate,
      bitrate_kbps: profile.bitrate_kbps,
      encoder_preference: profile.encoder_preference,
    });
  }

  function cancelRecordingProfileEdit() {
    setEditingProfileId(null);
    setProfileForm(settingsSnapshot?.settings.default_recording_profile ?? defaultProfileForm);
  }

  async function deleteRecordingProfile(profile: MediaProfile) {
    if (!config || !window.confirm(`Delete recording profile "${profile.name}"?`)) return;
    try {
      await StudioApi.deleteRecordingProfile(config, profile.id);
      await refreshProfiles();
      if (editingProfileId === profile.id) {
        cancelRecordingProfileEdit();
      }
      setError(null);
    } catch (error) {
      setError(error instanceof Error ? error.message : "Profile delete failed");
    }
  }

  function editStreamDestination(destination: StreamDestination) {
    setEditingDestinationId(destination.id);
    setDestinationForm({
      name: destination.name,
      platform: destination.platform,
      ingest_url: destination.ingest_url,
      stream_key: "",
      enabled: destination.enabled,
    });
  }

  function cancelStreamDestinationEdit() {
    setEditingDestinationId(null);
    setDestinationForm(defaultDestinationForm);
  }

  async function deleteStreamDestination(destination: StreamDestination) {
    if (!config || !window.confirm(`Delete stream destination "${destination.name}"?`)) return;
    try {
      await StudioApi.deleteStreamDestination(config, destination.id);
      await refreshProfiles();
      if (editingDestinationId === destination.id) {
        cancelStreamDestinationEdit();
      }
      setError(null);
    } catch (error) {
      setError(error instanceof Error ? error.message : "Destination delete failed");
    }
  }

  async function handleOpenSettingsWindow() {
    try {
      const { invoke } = await import("@tauri-apps/api/core");
      await invoke<void>("open_settings_window");
      setError(null);
    } catch (error) {
      setError(
        error instanceof Error
          ? error.message
          : "Configuration settings unavailable",
      );
    }
  }

  function applySettingsSnapshot(snapshot: LocalAppSettingsSnapshot) {
    setSettingsSnapshot(snapshot);
    setSettingsForm(snapshot.settings);
    setConfig((current) => ({
      apiUrl: snapshot.apiUrl,
      wsUrl: snapshot.wsUrl,
      configuredApiUrl: snapshot.configuredApiUrl,
      configuredWsUrl: snapshot.configuredWsUrl,
      bindAddr: hostFromUrl(
        snapshot.apiUrl,
        current?.bindAddr ?? "127.0.0.1:51287",
      ),
      configuredBindAddr: hostFromUrl(
        snapshot.configuredApiUrl,
        current?.configuredBindAddr ?? "127.0.0.1:51287",
      ),
      portFallbackActive: snapshot.portFallbackActive,
      discoveryFile: snapshot.discoveryFile,
      token: snapshot.settings.api_token,
      devAuthBypass: snapshot.settings.dev_auth_bypass,
    }));
  }

  async function handleSaveSettings(event: FormEvent) {
    event.preventDefault();
    try {
      const snapshot = await saveAppSettings(settingsForm);
      applySettingsSnapshot(snapshot);
      setProfileForm(snapshot.settings.default_recording_profile);
      await refreshCaptureContext();
      loadPreflightSnapshot().then(setPreflight).catch(() => undefined);
      setSettingsMessage(
        snapshot.restartRequired
          ? "Saved. Host or port changes apply after restart."
          : "Saved.",
      );
      setError(null);
    } catch (error) {
      setSettingsMessage(null);
      setError(error instanceof Error ? error.message : "Settings save failed");
    }
  }

  async function handleRegenerateApiToken() {
    try {
      const snapshot = await regenerateApiToken();
      applySettingsSnapshot(snapshot);
      setSettingsMessage("API token regenerated.");
      setError(null);
    } catch (error) {
      setSettingsMessage(null);
      setError(error instanceof Error ? error.message : "Token regeneration failed");
    }
  }

  async function handleOpenDataDirectory() {
    try {
      await openDataDirectory();
    } catch (error) {
      setError(
        error instanceof Error ? error.message : "Could not open data directory",
      );
    }
  }

  async function handleExportProfileBundle() {
    try {
      const result = await exportProfileBundle();
      setSettingsMessage(
        `Exported ${result.recordingProfiles} recording profiles and ${result.streamDestinations} destinations.`,
      );
      setError(null);
    } catch (error) {
      setSettingsMessage(null);
      setError(error instanceof Error ? error.message : "Profile export failed");
    }
  }

  async function handleImportProfileBundle() {
    if (!window.confirm("Import profile bundle from the app data directory?")) {
      return;
    }

    try {
      const result = await importProfileBundle();
      await refreshProfiles();
      setSettingsMessage(
        `Imported ${result.recordingProfiles} recording profiles and ${result.streamDestinations} destinations.`,
      );
      setError(null);
    } catch (error) {
      setSettingsMessage(null);
      setError(error instanceof Error ? error.message : "Profile import failed");
    }
  }

  const page = useMemo(() => {
    switch (section) {
      case "dashboard":
        return (
          <Dashboard
            activeDestination={activeDestination?.name ?? "None"}
            engine={activeStatus?.engine ?? "starting"}
            events={events}
            pipelinePlan={pipelinePlan}
            preflight={preflight}
            recordingActive={activeStatus?.recording_active ?? false}
            recordingPath={recordingPath ?? "No active recording"}
            streamActive={activeStatus?.stream_active ?? false}
          />
        );
      case "destinations":
        return (
          <DestinationsPage
            destinationForm={destinationForm}
            editingDestinationId={editingDestinationId}
            onCancelEdit={cancelStreamDestinationEdit}
            onCreate={createDestination}
            onDelete={deleteStreamDestination}
            onEdit={editStreamDestination}
            onFormChange={setDestinationForm}
            profiles={profiles}
          />
        );
      case "profiles":
        return (
          <RecordingProfilesPage
            editingProfileId={editingProfileId}
            onCancelEdit={cancelRecordingProfileEdit}
            onCreate={createRecordingProfile}
            onDelete={deleteRecordingProfile}
            onEdit={editRecordingProfile}
            onFormChange={setProfileForm}
            profileForm={profileForm}
            profiles={profiles}
          />
        );
      case "controls":
        return (
          <ControlsPage
            markerLabel={markerLabel}
            onMarkerLabelChange={setMarkerLabel}
            onStartRecording={() =>
              config &&
              runCommand(() => StudioApi.startRecording(config, selectedProfileId))
            }
            onStartStream={() =>
              config &&
              runCommand(() => StudioApi.startStream(config, selectedDestinationId))
            }
            onStopRecording={() =>
              config && runCommand(() => StudioApi.stopRecording(config))
            }
            onStopStream={() =>
              config && runCommand(() => StudioApi.stopStream(config))
            }
            onCreateMarker={() =>
              config &&
              StudioApi.createMarker(config, markerLabel).catch((error: Error) =>
                setError(error.message),
              )
            }
            profiles={profiles}
            recordingActive={activeStatus?.recording_active ?? false}
            selectedDestinationId={selectedDestinationId}
            selectedProfileId={selectedProfileId}
            setSelectedDestinationId={setSelectedDestinationId}
            setSelectedProfileId={setSelectedProfileId}
            streamActive={activeStatus?.stream_active ?? false}
          />
        );
      case "apps":
        return (
          <ConnectedAppsPage
            clients={clients}
            config={config}
            engine={activeStatus?.engine ?? "starting"}
            mediaRunnerInfo={mediaRunnerInfo}
          />
        );
      case "logs":
        return <LogsPage auditEntries={auditEntries} events={events} />;
    }
  }, [
    activeDestination?.name,
    activeStatus?.engine,
    activeStatus?.recording_active,
    activeStatus?.stream_active,
    auditEntries,
    clients,
    config,
    destinationForm,
    editingDestinationId,
    editingProfileId,
    events,
    markerLabel,
    mediaRunnerInfo,
    pipelinePlan,
    preflight,
    profileForm,
    profiles,
    recordingPath,
    section,
    selectedDestinationId,
    selectedProfileId,
  ]);

  if (isSettingsWindow) {
    return (
      <main className="settings-window">
        <header className="settings-window-header">
          <div className="brand-mark compact">
            <img alt="" src={logoUrl} />
          </div>
          <div>
            <p className="section-label">Configuration</p>
            <h1>Settings</h1>
          </div>
        </header>

        {error && (
          <div className="error-banner">
            <WifiOff size={17} />
            <span>{error}</span>
          </div>
        )}

        <SettingsPage
          config={config}
          engine={activeStatus?.engine ?? "starting"}
          health={health}
          logoUrl={logoUrl}
          captureInventory={captureInventory}
          mediaRunnerInfo={mediaRunnerInfo}
          mode={activeStatus?.mode ?? "dry_run"}
          onOpenCameraPrivacy={openCameraPrivacySettings}
          onExportProfileBundle={handleExportProfileBundle}
          onImportProfileBundle={handleImportProfileBundle}
          onOpenDataDirectory={handleOpenDataDirectory}
          onOpenMicrophonePrivacy={openMicrophonePrivacySettings}
          onOpenScreenRecordingPrivacy={openScreenRecordingPrivacySettings}
          onRegenerateToken={handleRegenerateApiToken}
          onRefreshCaptureContext={refreshCaptureContext}
          onSave={handleSaveSettings}
          onSettingsChange={setSettingsForm}
          permissionStatuses={permissionStatuses}
          settings={settingsForm}
          snapshot={settingsSnapshot}
          statusMessage={settingsMessage}
        />
      </main>
    );
  }

  return (
    <main className="shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <img alt="" src={logoUrl} />
          </div>
          <div>
            <h1>vaexcore studio</h1>
            <span>local control core</span>
          </div>
        </div>

        <nav>
          {navItems.map((item) => (
            <button
              className={item.id === section ? "nav-item active" : "nav-item"}
              key={item.id}
              onClick={() => setSection(item.id)}
              type="button"
            >
              {item.icon}
              <span>{item.label}</span>
            </button>
          ))}
        </nav>

        <div className="sidebar-footer">
          <div className="sidebar-status">
            <StatusDot active={!error} />
            <span>{error ? "API unavailable" : "API connected"}</span>
          </div>
          <button
            aria-label="Open Configuration Settings"
            className="icon-button sidebar-settings-button"
            onClick={handleOpenSettingsWindow}
            title="Configuration Settings"
            type="button"
          >
            <SettingsIcon size={16} />
          </button>
        </div>
      </aside>

      <section className="workspace">
        <header className="topbar">
          <div>
            <p className="section-label">{sectionTitle(section)}</p>
            <h2>{sectionHeading(section)}</h2>
          </div>
          <div className="topbar-status">
            <Pill tone={activeStatus?.recording_active ? "red" : "muted"}>
              <Video size={14} />
              {activeStatus?.recording_active ? "Recording" : "Recording idle"}
            </Pill>
            <Pill tone={activeStatus?.stream_active ? "green" : "muted"}>
              <Radio size={14} />
              {activeStatus?.stream_active ? "Streaming" : "Stream idle"}
            </Pill>
          </div>
        </header>

        {error && (
          <div className="error-banner">
            <WifiOff size={17} />
            <span>{error}</span>
          </div>
        )}

        {page}
      </section>
    </main>
  );
}

function Dashboard(props: {
  activeDestination: string;
  engine: string;
  events: StudioEvent[];
  pipelinePlan: MediaPipelinePlan | null;
  preflight: PreflightSnapshot | null;
  recordingActive: boolean;
  recordingPath: string;
  streamActive: boolean;
}) {
  return (
    <div className="stack">
      <div className="metric-grid">
        <Metric
          icon={<Video size={20} />}
          label="Recording"
          tone={props.recordingActive ? "red" : "muted"}
          value={props.recordingActive ? "Active" : "Idle"}
        />
        <Metric
          icon={<Radio size={20} />}
          label="Stream"
          tone={props.streamActive ? "green" : "muted"}
          value={props.streamActive ? "Live" : "Idle"}
        />
        <Metric
          icon={<MapPin size={20} />}
          label="Destination"
          tone="amber"
          value={props.activeDestination}
        />
        <Metric
          icon={<Terminal size={20} />}
          label="Engine"
          tone="muted"
          value={props.engine}
        />
      </div>

      <div className="two-column">
        <section className="panel wide">
          <PanelTitle title="Active Recording Path" />
          <code className="path-line">{props.recordingPath}</code>
          <div className="runtime-row">
            <RuntimeCounter label="Runtime" value="00:00:00" />
            <RuntimeCounter label="Dropped Frames" value="0" />
            <RuntimeCounter label="Encoder Load" value="dry-run" />
          </div>
        </section>
        <section className="panel">
          <PanelTitle title="Recent Events" />
          <EventList events={props.events.slice(0, 6)} compact />
        </section>
      </div>

      <div className="two-column">
        <section className="panel">
          <PanelTitle title="Preflight" />
          {props.preflight ? (
            <div className="check-list">
              <div className="check-row-compact">
                <strong>Overall</strong>
                <Pill tone={preflightTone(props.preflight.overall)}>
                  {preflightLabel(props.preflight.overall)}
                </Pill>
              </div>
              {props.preflight.checks.map((check) => (
                <div className="check-row-compact" key={check.id}>
                  <div>
                    <strong>{check.label}</strong>
                    <span>{check.detail}</span>
                  </div>
                  <Pill tone={preflightTone(check.status)}>
                    {preflightLabel(check.status)}
                  </Pill>
                </div>
              ))}
            </div>
          ) : (
            <div className="empty">Preflight pending</div>
          )}
        </section>

        <section className="panel">
          <PanelTitle title="Pipeline Plan" />
          {props.pipelinePlan ? (
            <div className="check-list">
              <div className="check-row-compact">
                <strong>{props.pipelinePlan.pipeline_name}</strong>
                <Pill tone={props.pipelinePlan.ready ? "green" : "red"}>
                  {props.pipelinePlan.ready ? "ready" : "blocked"}
                </Pill>
              </div>
              {props.pipelinePlan.steps.map((step) => (
                <div className="check-row-compact" key={step.id}>
                  <div>
                    <strong>{step.label}</strong>
                    <span>{step.detail}</span>
                  </div>
                  <Pill tone={stepTone(step.status)}>{step.status}</Pill>
                </div>
              ))}
            </div>
          ) : (
            <div className="empty">Plan pending</div>
          )}
        </section>
      </div>
    </div>
  );
}

function DestinationsPage(props: {
  destinationForm: StreamDestinationInput;
  editingDestinationId: string | null;
  onCancelEdit: () => void;
  onCreate: (event: FormEvent) => void;
  onDelete: (destination: StreamDestination) => void;
  onEdit: (destination: StreamDestination) => void;
  onFormChange: (value: StreamDestinationInput) => void;
  profiles: ProfilesSnapshot | null;
}) {
  const isEditing = props.editingDestinationId !== null;

  return (
    <div className="two-column">
      <section className="panel wide">
        <PanelTitle title="Stream Destinations" />
        <div className="table">
          {(props.profiles?.stream_destinations ?? []).map((destination) => (
            <div className="table-row" key={destination.id}>
              <div>
                <strong>{destination.name}</strong>
                <span>{destination.ingest_url}</span>
              </div>
              <Pill tone={destination.enabled ? "green" : "muted"}>
                {platformLabels[destination.platform]}
              </Pill>
              <Pill tone={destination.stream_key_ref ? "amber" : "muted"}>
                {destination.stream_key_ref ? "key stored" : "no key"}
              </Pill>
              <div className="table-actions">
                <button
                  aria-label={`Edit ${destination.name}`}
                  className="icon-button"
                  onClick={() => props.onEdit(destination)}
                  title={`Edit ${destination.name}`}
                  type="button"
                >
                  <Pencil size={15} />
                </button>
                <button
                  aria-label={`Delete ${destination.name}`}
                  className="icon-button danger"
                  onClick={() => props.onDelete(destination)}
                  title={`Delete ${destination.name}`}
                  type="button"
                >
                  <Trash2 size={15} />
                </button>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="panel">
        <PanelTitle title={isEditing ? "Edit Destination" : "Create Destination"} />
        <form className="form" onSubmit={props.onCreate}>
          <TextInput
            label="Name"
            value={props.destinationForm.name}
            onChange={(name) =>
              props.onFormChange({ ...props.destinationForm, name })
            }
          />
          <label>
            Platform
            <select
              value={props.destinationForm.platform}
              onChange={(event) =>
                props.onFormChange({
                  ...props.destinationForm,
                  platform: event.target.value as PlatformKind,
                })
              }
            >
              {Object.entries(platformLabels).map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          <TextInput
            label="Ingest URL"
            value={props.destinationForm.ingest_url ?? ""}
            onChange={(ingest_url) =>
              props.onFormChange({ ...props.destinationForm, ingest_url })
            }
          />
          <TextInput
            label="Stream Key"
            type="password"
            value={props.destinationForm.stream_key ?? ""}
            onChange={(stream_key) =>
              props.onFormChange({ ...props.destinationForm, stream_key })
            }
          />
          {isEditing && (
            <p className="form-hint">Leave blank to keep the stored key.</p>
          )}
          <label className="check-row">
            <input
              checked={props.destinationForm.enabled ?? true}
              onChange={(event) =>
                props.onFormChange({
                  ...props.destinationForm,
                  enabled: event.target.checked,
                })
              }
              type="checkbox"
            />
            Enabled
          </label>
          <div className="button-row">
            <button className="primary-button" type="submit">
              {isEditing ? <CheckCircle2 size={16} /> : <Plus size={16} />}
              {isEditing ? "Save Destination" : "Add Destination"}
            </button>
            {isEditing && (
              <button
                className="secondary-button"
                onClick={props.onCancelEdit}
                type="button"
              >
                <X size={16} />
                Cancel
              </button>
            )}
          </div>
        </form>
      </section>
    </div>
  );
}

function RecordingProfilesPage(props: {
  editingProfileId: string | null;
  onCancelEdit: () => void;
  onCreate: (event: FormEvent) => void;
  onDelete: (profile: MediaProfile) => void;
  onEdit: (profile: MediaProfile) => void;
  onFormChange: (value: MediaProfileInput) => void;
  profileForm: MediaProfileInput;
  profiles: ProfilesSnapshot | null;
}) {
  const isEditing = props.editingProfileId !== null;

  return (
    <div className="two-column">
      <section className="panel wide">
        <PanelTitle title="Recording Profiles" />
        <div className="table">
          {(props.profiles?.recording_profiles ?? []).map((profile) => (
            <div className="table-row" key={profile.id}>
              <div>
                <strong>{profile.name}</strong>
                <span>
                  {profile.resolution.width}x{profile.resolution.height} -{" "}
                  {profile.framerate} fps - {profile.bitrate_kbps} kbps
                </span>
              </div>
              <Pill tone="amber">{profile.container}</Pill>
              <code>{profile.output_folder}</code>
              <div className="table-actions">
                <button
                  aria-label={`Edit ${profile.name}`}
                  className="icon-button"
                  onClick={() => props.onEdit(profile)}
                  title={`Edit ${profile.name}`}
                  type="button"
                >
                  <Pencil size={15} />
                </button>
                <button
                  aria-label={`Delete ${profile.name}`}
                  className="icon-button danger"
                  onClick={() => props.onDelete(profile)}
                  title={`Delete ${profile.name}`}
                  type="button"
                >
                  <Trash2 size={15} />
                </button>
              </div>
            </div>
          ))}
        </div>
      </section>

      <section className="panel">
        <PanelTitle title={isEditing ? "Edit Profile" : "Create Profile"} />
        <form className="form" onSubmit={props.onCreate}>
          <TextInput
            label="Name"
            value={props.profileForm.name}
            onChange={(name) => props.onFormChange({ ...props.profileForm, name })}
          />
          <TextInput
            label="Output Folder"
            value={props.profileForm.output_folder}
            onChange={(output_folder) =>
              props.onFormChange({ ...props.profileForm, output_folder })
            }
          />
          <TextInput
            label="Filename Pattern"
            value={props.profileForm.filename_pattern}
            onChange={(filename_pattern) =>
              props.onFormChange({ ...props.profileForm, filename_pattern })
            }
          />
          <div className="form-grid">
            <NumberInput
              label="Width"
              value={props.profileForm.resolution.width}
              onChange={(width) =>
                props.onFormChange({
                  ...props.profileForm,
                  resolution: { ...props.profileForm.resolution, width },
                })
              }
            />
            <NumberInput
              label="Height"
              value={props.profileForm.resolution.height}
              onChange={(height) =>
                props.onFormChange({
                  ...props.profileForm,
                  resolution: { ...props.profileForm.resolution, height },
                })
              }
            />
          </div>
          <div className="form-grid">
            <NumberInput
              label="Framerate"
              value={props.profileForm.framerate}
              onChange={(framerate) =>
                props.onFormChange({ ...props.profileForm, framerate })
              }
            />
            <NumberInput
              label="Bitrate"
              value={props.profileForm.bitrate_kbps}
              onChange={(bitrate_kbps) =>
                props.onFormChange({ ...props.profileForm, bitrate_kbps })
              }
            />
          </div>
          <label>
            Container
            <select
              value={props.profileForm.container}
              onChange={(event) =>
                props.onFormChange({
                  ...props.profileForm,
                  container: event.target.value as RecordingContainer,
                })
              }
            >
              <option value="mkv">MKV</option>
              <option value="mp4">MP4</option>
            </select>
          </label>
          <label>
            Encoder
            <select
              value={
                typeof props.profileForm.encoder_preference === "string"
                  ? props.profileForm.encoder_preference
                  : "auto"
              }
              onChange={(event) =>
                props.onFormChange({
                  ...props.profileForm,
                  encoder_preference: event.target.value as "auto" | "hardware" | "software",
                })
              }
            >
              <option value="auto">Auto</option>
              <option value="hardware">Hardware</option>
              <option value="software">Software</option>
            </select>
          </label>
          <div className="button-row">
            <button className="primary-button" type="submit">
              {isEditing ? <CheckCircle2 size={16} /> : <Plus size={16} />}
              {isEditing ? "Save Profile" : "Add Profile"}
            </button>
            {isEditing && (
              <button
                className="secondary-button"
                onClick={props.onCancelEdit}
                type="button"
              >
                <X size={16} />
                Cancel
              </button>
            )}
          </div>
        </form>
      </section>
    </div>
  );
}

function ControlsPage(props: {
  markerLabel: string;
  onCreateMarker: () => void;
  onMarkerLabelChange: (value: string) => void;
  onStartRecording: () => void;
  onStartStream: () => void;
  onStopRecording: () => void;
  onStopStream: () => void;
  profiles: ProfilesSnapshot | null;
  recordingActive: boolean;
  selectedDestinationId: string | undefined;
  selectedProfileId: string | undefined;
  setSelectedDestinationId: (value: string) => void;
  setSelectedProfileId: (value: string) => void;
  streamActive: boolean;
}) {
  return (
    <div className="control-grid">
      <section className="panel">
        <PanelTitle title="Recording" />
        <label>
          Profile
          <select
            value={props.selectedProfileId}
            onChange={(event) => props.setSelectedProfileId(event.target.value)}
          >
            {(props.profiles?.recording_profiles ?? []).map((profile) => (
              <option key={profile.id} value={profile.id}>
                {profile.name}
              </option>
            ))}
          </select>
        </label>
        <div className="button-row">
          <button
            className="primary-button danger"
            disabled={props.recordingActive}
            onClick={props.onStartRecording}
            type="button"
          >
            <Play size={16} />
            Start Recording
          </button>
          <button
            className="secondary-button"
            disabled={!props.recordingActive}
            onClick={props.onStopRecording}
            type="button"
          >
            <Square size={16} />
            Stop
          </button>
        </div>
      </section>

      <section className="panel">
        <PanelTitle title="Stream" />
        <label>
          Destination
          <select
            value={props.selectedDestinationId}
            onChange={(event) =>
              props.setSelectedDestinationId(event.target.value)
            }
          >
            {(props.profiles?.stream_destinations ?? []).map((destination) => (
              <option key={destination.id} value={destination.id}>
                {destination.name}
              </option>
            ))}
          </select>
        </label>
        <div className="button-row">
          <button
            className="primary-button"
            disabled={props.streamActive}
            onClick={props.onStartStream}
            type="button"
          >
            <Play size={16} />
            Start Stream
          </button>
          <button
            className="secondary-button"
            disabled={!props.streamActive}
            onClick={props.onStopStream}
            type="button"
          >
            <Square size={16} />
            Stop
          </button>
        </div>
      </section>

      <section className="panel">
        <PanelTitle title="Marker" />
        <TextInput
          label="Label"
          value={props.markerLabel}
          onChange={props.onMarkerLabelChange}
        />
        <button
          className="secondary-button full"
          onClick={props.onCreateMarker}
          type="button"
        >
          <MapPin size={16} />
          Create Marker
        </button>
      </section>
    </div>
  );
}

function ConnectedAppsPage(props: {
  clients: ConnectedClient[];
  config: RuntimeApiConfig | null;
  engine: string;
  mediaRunnerInfo: MediaRunnerInfo | null;
}) {
  const apiUrl = props.config?.apiUrl ?? "http://127.0.0.1:51287";
  const wsUrl = props.config?.wsUrl ?? "ws://127.0.0.1:51287/events";
  const configuredApiUrl = props.config?.configuredApiUrl ?? apiUrl;
  const token = props.config?.token ?? "dev-auth-bypass";
  const runnerState = mediaRunnerState(props.mediaRunnerInfo, props.engine);

  return (
    <div className="stack">
      <section className="panel">
        <PanelTitle title="Local Endpoints" />
        <CopyLine label="HTTP API URL" value={apiUrl} />
        <CopyLine label="WebSocket URL" value={wsUrl} />
        {props.config?.portFallbackActive && (
          <CopyLine label="Configured API URL" value={configuredApiUrl} />
        )}
        {props.config?.discoveryFile && (
          <CopyLine label="Discovery File" value={props.config.discoveryFile} />
        )}
        <CopyLine label="API Token" secret value={token} />
      </section>
      <section className="panel">
        <PanelTitle title="Local Runtime" />
        <KeyValue label="Media Runner" value={runnerState} />
        <KeyValue
          label="Sidecar Bundle"
          value={props.mediaRunnerInfo?.bundled ? "bundled" : "not bundled"}
        />
        <KeyValue
          label="Status Endpoint"
          value={props.mediaRunnerInfo?.statusAddr ?? "inactive"}
        />
      </section>
      <section className="panel">
        <PanelTitle title="Recent Clients" />
        {props.clients.length === 0 ? (
          <div className="empty">No clients yet</div>
        ) : (
          <div className="table">
            {props.clients.map((client) => (
              <div className="table-row" key={`${client.kind}-${client.id}`}>
                <div>
                  <strong>{client.name}</strong>
                  <span>{client.last_path ?? "local API"}</span>
                </div>
                <Pill tone={client.kind === "websocket" ? "green" : "amber"}>
                  {client.kind}
                </Pill>
                <Pill tone="muted">{client.request_count} req</Pill>
                <span>{new Date(client.last_seen_at).toLocaleTimeString()}</span>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

function LogsPage(props: {
  auditEntries: AuditLogEntry[];
  events: StudioEvent[];
}) {
  return (
    <div className="stack">
      <section className="panel">
        <PanelTitle title="Command Audit" />
        <AuditList entries={props.auditEntries} />
      </section>
      <section className="panel">
        <PanelTitle title="Event Log" />
        <EventList events={props.events} />
      </section>
    </div>
  );
}

function SettingsPage(props: {
  captureInventory: CaptureSourceInventory | null;
  config: RuntimeApiConfig | null;
  engine: string;
  health: HealthResponse | null;
  logoUrl: string;
  mediaRunnerInfo: MediaRunnerInfo | null;
  mode: string;
  onExportProfileBundle: () => void;
  onImportProfileBundle: () => void;
  onOpenCameraPrivacy: () => Promise<void>;
  onOpenDataDirectory: () => void;
  onOpenMicrophonePrivacy: () => Promise<void>;
  onOpenScreenRecordingPrivacy: () => Promise<void>;
  onRegenerateToken: () => void;
  onRefreshCaptureContext: () => Promise<void>;
  onSave: (event: FormEvent) => void;
  onSettingsChange: (settings: AppSettings) => void;
  permissionStatuses: {
    camera: PermissionStatus | null;
    microphone: PermissionStatus | null;
  };
  settings: AppSettings;
  snapshot: LocalAppSettingsSnapshot | null;
  statusMessage: string | null;
}) {
  const profile = props.settings.default_recording_profile;
  const token = props.settings.api_token ?? "not configured";

  function updateSettings(update: Partial<AppSettings>) {
    props.onSettingsChange({ ...props.settings, ...update });
  }

  function updateDefaultProfile(update: Partial<MediaProfileInput>) {
    updateSettings({
      default_recording_profile: {
        ...profile,
        ...update,
      },
    });
  }

  function updateCaptureSource(
    candidate: CaptureSourceCandidate,
    enabled: boolean,
  ) {
    const existing = props.settings.capture_sources.find(
      (source) => source.id === candidate.id,
    );
    const nextSource: CaptureSourceSelection = {
      id: candidate.id,
      kind: candidate.kind,
      name: candidate.name,
      enabled,
    };
    const nextSources = existing
      ? props.settings.capture_sources.map((source) =>
          source.id === candidate.id ? { ...source, enabled } : source,
        )
      : [...props.settings.capture_sources, nextSource];

    updateSettings({ capture_sources: nextSources });
  }

  const sourceCandidates =
    props.captureInventory?.candidates ??
    props.settings.capture_sources.map((source) => ({
      id: source.id,
      kind: source.kind,
      name: source.name,
      available: true,
      notes: null,
    }));

  return (
    <form className="settings-grid" onSubmit={props.onSave}>
      <section className="panel identity-panel">
        <img alt="" src={props.logoUrl} />
        <div>
          <PanelTitle title="Identity" />
          <KeyValue label="Product" value="vaexcore studio" />
          <KeyValue label="Role" value="local foundation layer" />
        </div>
      </section>

      <section className="panel">
        <PanelTitle title="Local API" />
        <TextInput
          label="Host"
          value={props.settings.api_host}
          onChange={(api_host) => updateSettings({ api_host })}
        />
        <NumberInput
          label="Port"
          value={props.settings.api_port}
          onChange={(api_port) => updateSettings({ api_port })}
        />
        <label className="check-row">
          <input
            checked={props.settings.dev_auth_bypass}
            onChange={(event) =>
              updateSettings({ dev_auth_bypass: event.target.checked })
            }
            type="checkbox"
          />
          Dev auth bypass
        </label>
        <CopyLine label="API Token" secret value={token} />
        <div className="button-row">
          <button
            className="secondary-button"
            onClick={props.onRegenerateToken}
            type="button"
          >
            Regenerate Token
          </button>
          {props.snapshot?.restartRequired && (
            <Pill tone="amber">restart required</Pill>
          )}
        </div>
      </section>

      <section className="panel">
        <PanelTitle title="Runtime" />
        <KeyValue label="Engine" value={props.engine} />
        <KeyValue label="Mode" value={props.mode} />
        <KeyValue
          label="Active API"
          value={props.config?.bindAddr ?? "127.0.0.1:51287"}
        />
        <KeyValue
          label="Configured API"
          value={props.config?.configuredBindAddr ?? "127.0.0.1:51287"}
        />
        <KeyValue
          label="Port Fallback"
          value={props.config?.portFallbackActive ? "active" : "inactive"}
        />
        <KeyValue
          label="Media Runner"
          value={mediaRunnerState(props.mediaRunnerInfo, props.engine)}
        />
        <KeyValue
          label="Sidecar Bundle"
          value={props.mediaRunnerInfo?.bundled ? "bundled" : "not bundled"}
        />
        <KeyValue
          label="Service"
          value={props.health?.service ?? "vaexcore studio"}
        />
        <KeyValue label="Version" value={props.health?.version ?? "0.1.0"} />
        {props.snapshot?.discoveryFile && (
          <CopyLine label="Discovery File" value={props.snapshot.discoveryFile} />
        )}
        {props.snapshot?.logDir && (
          <CopyLine label="Log Directory" value={props.snapshot.logDir} />
        )}
      </section>

      <section className="panel">
        <PanelTitle title="Security" />
        <KeyValue
          label="Auth Required"
          value={props.health?.auth_required ? "yes" : "no"}
        />
        <KeyValue
          label="Dev Auth Bypass"
          value={props.config?.devAuthBypass ? "enabled" : "disabled"}
        />
        <KeyValue
          label="Token"
          value={props.config?.token ? "generated" : "not configured"}
        />
      </section>

      <section className="panel settings-wide-panel">
        <PanelTitle
          action={
            <button
              className="secondary-button compact"
              onClick={() => props.onRefreshCaptureContext().catch(() => undefined)}
              type="button"
            >
              Refresh
            </button>
          }
          title="Capture Sources"
        />
        <div className="permission-grid">
          <PermissionStatusLine
            label="Camera"
            onOpen={() => props.onOpenCameraPrivacy().catch(() => undefined)}
            status={props.permissionStatuses.camera}
          />
          <PermissionStatusLine
            label="Microphone"
            onOpen={() => props.onOpenMicrophonePrivacy().catch(() => undefined)}
            status={props.permissionStatuses.microphone}
          />
          <div className="permission-line">
            <div>
              <strong>Screen Recording</strong>
              <span>Required for display and window capture.</span>
            </div>
            <button
              className="secondary-button compact"
              onClick={() =>
                props.onOpenScreenRecordingPrivacy().catch(() => undefined)
              }
              type="button"
            >
              Open Privacy
            </button>
          </div>
        </div>
        <div className="source-grid">
          {sourceCandidates.map((candidate) => {
            const selected = props.settings.capture_sources.find(
              (source) => source.id === candidate.id,
            );
            const checked = selected?.enabled ?? false;
            return (
              <label className="source-option" key={candidate.id}>
                <input
                  checked={checked}
                  disabled={!candidate.available}
                  onChange={(event) =>
                    updateCaptureSource(candidate, event.target.checked)
                  }
                  type="checkbox"
                />
                <div>
                  <strong>{candidate.name}</strong>
                  <span>{captureSourceKindLabel(candidate.kind)}</span>
                  {candidate.notes && <small>{candidate.notes}</small>}
                </div>
              </label>
            );
          })}
        </div>
      </section>

      <section className="panel settings-wide-panel">
        <PanelTitle title="Default Recording Profile" />
        <div className="form-grid">
          <TextInput
            label="Name"
            value={profile.name}
            onChange={(name) => updateDefaultProfile({ name })}
          />
          <TextInput
            label="Output Folder"
            value={profile.output_folder}
            onChange={(output_folder) =>
              updateDefaultProfile({ output_folder })
            }
          />
        </div>
        <TextInput
          label="Filename Pattern"
          value={profile.filename_pattern}
          onChange={(filename_pattern) =>
            updateDefaultProfile({ filename_pattern })
          }
        />
        <div className="form-grid">
          <NumberInput
            label="Width"
            value={profile.resolution.width}
            onChange={(width) =>
              updateDefaultProfile({
                resolution: { ...profile.resolution, width },
              })
            }
          />
          <NumberInput
            label="Height"
            value={profile.resolution.height}
            onChange={(height) =>
              updateDefaultProfile({
                resolution: { ...profile.resolution, height },
              })
            }
          />
        </div>
        <div className="form-grid">
          <NumberInput
            label="Framerate"
            value={profile.framerate}
            onChange={(framerate) => updateDefaultProfile({ framerate })}
          />
          <NumberInput
            label="Bitrate"
            value={profile.bitrate_kbps}
            onChange={(bitrate_kbps) => updateDefaultProfile({ bitrate_kbps })}
          />
        </div>
        <div className="form-grid">
          <label>
            Container
            <select
              value={profile.container}
              onChange={(event) =>
                updateDefaultProfile({
                  container: event.target.value as RecordingContainer,
                })
              }
            >
              <option value="mkv">MKV</option>
              <option value="mp4">MP4</option>
            </select>
          </label>
          <label>
            Encoder
            <select
              value={
                typeof profile.encoder_preference === "string"
                  ? profile.encoder_preference
                  : "auto"
              }
              onChange={(event) =>
                updateDefaultProfile({
                  encoder_preference: event.target.value as
                    | "auto"
                    | "hardware"
                    | "software",
                })
              }
            >
              <option value="auto">Auto</option>
              <option value="hardware">Hardware</option>
              <option value="software">Software</option>
            </select>
          </label>
        </div>
      </section>

      <section className="panel">
        <PanelTitle title="Storage" />
        <CopyLine label="Data Directory" value={props.snapshot?.dataDir ?? ""} />
        <CopyLine
          label="Database"
          value={props.snapshot?.databasePath ?? ""}
        />
        <CopyLine
          label="Pipeline Plan"
          value={props.snapshot?.pipelinePlanPath ?? ""}
        />
        <CopyLine
          label="Pipeline Config"
          value={props.snapshot?.pipelineConfigPath ?? ""}
        />
        <button
          className="secondary-button full"
          onClick={props.onOpenDataDirectory}
          type="button"
        >
          Open Data Directory
        </button>
        <div className="button-row">
          <button
            className="secondary-button"
            onClick={props.onExportProfileBundle}
            type="button"
          >
            Export Profiles
          </button>
          <button
            className="secondary-button"
            onClick={props.onImportProfileBundle}
            type="button"
          >
            Import Profiles
          </button>
        </div>
      </section>

      <section className="panel">
        <PanelTitle title="Diagnostics" />
        <label>
          Log Level
          <select
            value={props.settings.log_level}
            onChange={(event) =>
              updateSettings({
                log_level: event.target.value as AppSettings["log_level"],
              })
            }
          >
            <option value="trace">Trace</option>
            <option value="debug">Debug</option>
            <option value="info">Info</option>
            <option value="warn">Warn</option>
            <option value="error">Error</option>
          </select>
        </label>
        <p className="muted-note">
          Log level changes are persisted and apply on next launch.
        </p>
      </section>

      <section className="panel settings-actions-panel">
        {props.statusMessage && <Pill tone="green">{props.statusMessage}</Pill>}
        <button className="primary-button" type="submit">
          Save Settings
        </button>
      </section>
    </form>
  );
}

function Metric(props: {
  icon: ReactNode;
  label: string;
  tone: "green" | "red" | "amber" | "muted";
  value: string;
}) {
  return (
    <section className={`metric ${props.tone}`}>
      <div className="metric-icon">{props.icon}</div>
      <span>{props.label}</span>
      <strong>{props.value}</strong>
    </section>
  );
}

function RuntimeCounter(props: { label: string; value: string }) {
  return (
    <div>
      <span>{props.label}</span>
      <strong>{props.value}</strong>
    </div>
  );
}

function PermissionStatusLine(props: {
  label: string;
  onOpen: () => void;
  status: PermissionStatus | null;
}) {
  const status = props.status?.status ?? "unknown";
  return (
    <div className="permission-line">
      <div>
        <strong>{props.label}</strong>
        <span>{props.status?.detail ?? "Permission status pending."}</span>
      </div>
      <div className="permission-actions">
        <Pill tone={permissionTone(status)}>{permissionLabel(status)}</Pill>
        <button
          className="secondary-button compact"
          onClick={props.onOpen}
          type="button"
        >
          Open Privacy
        </button>
      </div>
    </div>
  );
}

function PanelTitle(props: { action?: ReactNode; title: string }) {
  return (
    <div className="panel-title">
      <h3>{props.title}</h3>
      {props.action}
    </div>
  );
}

function AuditList(props: { entries: AuditLogEntry[] }) {
  if (props.entries.length === 0) {
    return <div className="empty">No commands yet</div>;
  }

  return (
    <div className="event-list">
      {props.entries.map((entry) => (
        <div className="audit-row" key={entry.id}>
          <div>
            <strong>{entry.action}</strong>
            <span>
              {entry.method} {entry.path}
            </span>
            <code>{entry.request_id}</code>
          </div>
          <div className="audit-meta">
            <Pill tone={entry.ok ? "green" : "red"}>{entry.status_code}</Pill>
            <span>{entry.client_name ?? "Local client"}</span>
            <span>{new Date(entry.created_at).toLocaleTimeString()}</span>
          </div>
        </div>
      ))}
    </div>
  );
}

function EventList(props: { compact?: boolean; events: StudioEvent[] }) {
  if (props.events.length === 0) {
    return <div className="empty">No events yet</div>;
  }

  return (
    <div className={props.compact ? "event-list compact" : "event-list"}>
      {props.events.map((event) => (
        <div className="event-row" key={event.id}>
          <CheckCircle2 size={15} />
          <div>
            <strong>{event.type}</strong>
            <span>{new Date(event.timestamp).toLocaleTimeString()}</span>
          </div>
        </div>
      ))}
    </div>
  );
}

function CopyLine(props: { label: string; secret?: boolean; value: string }) {
  return (
    <div className="copy-line">
      <div>
        <span>{props.label}</span>
        <code>{props.secret ? mask(props.value) : props.value}</code>
      </div>
      <button
        aria-label={`Copy ${props.label}`}
        className="icon-button"
        onClick={() => navigator.clipboard.writeText(props.value)}
        title={`Copy ${props.label}`}
        type="button"
      >
        <Copy size={16} />
      </button>
    </div>
  );
}

function KeyValue(props: { label: string; value: string }) {
  return (
    <div className="key-value">
      <span>{props.label}</span>
      <strong>{props.value}</strong>
    </div>
  );
}

function Pill(props: {
  children: ReactNode;
  tone: "green" | "red" | "amber" | "muted";
}) {
  return <span className={`pill ${props.tone}`}>{props.children}</span>;
}

function StatusDot(props: { active: boolean }) {
  return <span className={props.active ? "status-dot active" : "status-dot"} />;
}

function TextInput(props: {
  label: string;
  onChange: (value: string) => void;
  type?: string;
  value: string;
}) {
  return (
    <label>
      {props.label}
      <input
        type={props.type ?? "text"}
        value={props.value}
        onChange={(event) => props.onChange(event.target.value)}
      />
    </label>
  );
}

function NumberInput(props: {
  label: string;
  onChange: (value: number) => void;
  value: number;
}) {
  return (
    <label>
      {props.label}
      <input
        min={1}
        type="number"
        value={props.value}
        onChange={(event) => props.onChange(Number(event.target.value))}
      />
    </label>
  );
}

function isSection(value: unknown): value is Section {
  return typeof value === "string" && sectionIds.includes(value as Section);
}

function sectionTitle(section: Section): string {
  return section
    .split("_")
    .join(" ")
    .replace(/^\w/, (letter) => letter.toUpperCase());
}

function sectionHeading(section: Section): string {
  switch (section) {
    case "dashboard":
      return "Studio Control Surface";
    case "destinations":
      return "Stream Destinations";
    case "profiles":
      return "Recording Profiles";
    case "controls":
      return "Media Controls";
    case "apps":
      return "Connected Apps";
    case "logs":
      return "Event Logs";
  }
}

function mergeEvents(events: StudioEvent[]): StudioEvent[] {
  const seen = new Set<string>();
  return events
    .filter((event) => {
      if (seen.has(event.id)) return false;
      seen.add(event.id);
      return true;
    })
    .slice(0, 100);
}

function mediaRunnerState(
  info: MediaRunnerInfo | null,
  engine: string,
): string {
  if (info?.running && info.bundled) return "bundled, running";
  if (info?.running) return "running";
  if (engine === "starting") return "unavailable";
  return "fallback dry-run";
}

function preflightTone(status: PreflightStatus): "green" | "red" | "amber" | "muted" {
  switch (status) {
    case "ready":
      return "green";
    case "blocked":
      return "red";
    case "warning":
    case "unknown":
      return "amber";
    case "not_required":
      return "muted";
  }
}

function preflightLabel(status: PreflightStatus): string {
  return status.replace("_", " ");
}

function permissionTone(
  status: PermissionStatus["status"],
): "green" | "red" | "amber" | "muted" {
  if (status === "authorized") return "green";
  if (status === "denied" || status === "restricted") return "red";
  if (status === "not_determined") return "amber";
  return "muted";
}

function permissionLabel(status: PermissionStatus["status"]): string {
  return status.replace("_", " ");
}

function captureSourceKindLabel(kind: CaptureSourceKind): string {
  return kind.replace("_", " ");
}

function stepTone(status: string): "green" | "red" | "amber" | "muted" {
  switch (status) {
    case "ready":
      return "green";
    case "blocked":
      return "red";
    case "warning":
      return "amber";
    default:
      return "muted";
  }
}

function mask(value: string): string {
  if (value.length <= 8) return "********";
  return `${value.slice(0, 4)}****${value.slice(-4)}`;
}

export default App;
