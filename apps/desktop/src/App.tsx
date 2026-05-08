import {
  Activity,
  ArrowDown,
  ArrowUp,
  Cable,
  CheckCircle2,
  Copy,
  Eye,
  EyeOff,
  FileVideo,
  Globe,
  Group,
  Image as ImageIcon,
  Layers,
  Link2,
  Lock,
  MapPin,
  Mic,
  Monitor,
  Pencil,
  Play,
  Plus,
  Radio,
  RefreshCw,
  ScrollText,
  Settings as SettingsIcon,
  SlidersHorizontal,
  Square,
  Terminal,
  Trash2,
  Type,
  Unlock,
  Video,
  WifiOff,
  X,
} from "lucide-react";
import {
  CSSProperties,
  FormEvent,
  PointerEvent as ReactPointerEvent,
  ReactNode,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type {
  AppSettings,
  AuditLogEntry,
  CaptureSourceCandidate,
  CaptureSourceInventory,
  CaptureSourceKind,
  CaptureSourceSelection,
  CompositorGraph,
  CompositorNode,
  ConnectedClient,
  HealthResponse,
  Marker,
  MediaPipelinePlan,
  MediaProfile,
  MediaProfileInput,
  PlatformKind,
  PreflightSnapshot,
  PreflightStatus,
  ProfilesSnapshot,
  RecordingContainer,
  RecordingHistoryEntry,
  Scene,
  SceneCollection,
  SceneCrop,
  ScenePoint,
  SceneSize,
  SceneSource,
  SceneSourceKind,
  StudioEvent,
  StudioStatus,
  StreamDestination,
  StreamDestinationInput,
} from "@vaexcore/shared-types";
import {
  bindSceneCollectionCaptureInventory,
  buildCompositorGraph,
  createDefaultSceneCollection,
  createDefaultSceneSource,
  platformLabels,
  sceneSourceKindLabels,
  validateCompositorGraph,
  validateSceneCollection,
} from "@vaexcore/shared-types";
import {
  eventSocketUrl,
  exportProfileBundle,
  fetchTwitchBroadcastReadinessFromConsole,
  fetchTwitchStreamKeyFromConsole,
  handoffRecordingToPulse,
  importProfileBundle,
  launchVaexcoreSuite,
  LocalAppSettingsSnapshot,
  loadCameraPermissionStatus,
  loadCaptureSourceInventory,
  loadMediaRunnerInfo,
  loadMicrophonePermissionStatus,
  loadPreflightSnapshot,
  loadRuntimeConfig,
  loadAppSettings,
  loadSuiteStatus,
  loadSuiteSession,
  loadSuiteTimeline,
  MediaRunnerInfo,
  openDataDirectory,
  openCameraPrivacySettings,
  openMicrophonePrivacySettings,
  openScreenRecordingPrivacySettings,
  PermissionStatus,
  regenerateApiToken,
  recordSuiteTimelineEvent,
  RuntimeApiConfig,
  saveAppSettings,
  sendSuiteCommand,
  StudioApi,
  startSuiteSession,
  SuiteAppStatus,
  SuiteLaunchResult,
  SuiteSession,
  SuiteTimelineEvent,
  TwitchBroadcastReadiness,
} from "./api";
import logoUrl from "./assets/brand/vaexcore-studio-logo.jpg";

type Section =
  | "dashboard"
  | "designer"
  | "destinations"
  | "profiles"
  | "controls"
  | "apps"
  | "logs";

type SuiteTimelineItem = {
  id: string;
  kind: "presence" | "recording" | "marker" | "event";
  title: string;
  detail: string;
  timestamp: string;
  source: string;
};

type SceneSourcePatch = Partial<
  Pick<
    SceneSource,
    "name" | "opacity" | "rotation_degrees" | "visible" | "locked" | "z_index"
  >
> & {
  position?: Partial<ScenePoint>;
  size?: Partial<SceneSize>;
  crop?: Partial<SceneCrop>;
  config?: Record<string, unknown>;
};

type DesignerDragState = {
  mode: "move" | "resize";
  pointerId: number;
  sourceId: string;
  startClientX: number;
  startClientY: number;
  startPosition: ScenePoint;
  startSize: SceneSize;
};

const sectionIds: readonly Section[] = [
  "dashboard",
  "designer",
  "destinations",
  "profiles",
  "controls",
  "apps",
  "logs",
];

const navItems: Array<{ id: Section; label: string; icon: ReactNode }> = [
  { id: "dashboard", label: "Control Room", icon: <Activity size={17} /> },
  { id: "designer", label: "Designer", icon: <Layers size={17} /> },
  { id: "destinations", label: "Broadcast Destinations", icon: <Radio size={17} /> },
  { id: "profiles", label: "Recording Profiles", icon: <FileVideo size={17} /> },
  { id: "controls", label: "Broadcast Setup", icon: <SlidersHorizontal size={17} /> },
  { id: "apps", label: "Suite", icon: <Cable size={17} /> },
  { id: "logs", label: "Event Log", icon: <ScrollText size={17} /> },
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
  const [section, setSection] = useState<Section>(() => initialSection());
  const [config, setConfig] = useState<RuntimeApiConfig | null>(null);
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [status, setStatus] = useState<StudioStatus | null>(null);
  const [profiles, setProfiles] = useState<ProfilesSnapshot | null>(null);
  const [events, setEvents] = useState<StudioEvent[]>([]);
  const [clients, setClients] = useState<ConnectedClient[]>([]);
  const [auditEntries, setAuditEntries] = useState<AuditLogEntry[]>([]);
  const [recentRecordings, setRecentRecordings] = useState<RecordingHistoryEntry[]>([]);
  const [recentMarkers, setRecentMarkers] = useState<Marker[]>([]);
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
  const [captureSourceSaving, setCaptureSourceSaving] = useState(false);
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
  const [suiteLaunchStatus, setSuiteLaunchStatus] = useState<string | null>(null);
  const [suiteStatus, setSuiteStatus] = useState<SuiteAppStatus[]>([]);
  const [suiteSession, setSuiteSession] = useState<SuiteSession | null>(null);
  const [persistedSuiteTimeline, setPersistedSuiteTimeline] = useState<
    SuiteTimelineEvent[]
  >([]);
  const [twitchReadiness, setTwitchReadiness] =
    useState<TwitchBroadcastReadiness | null>(null);
  const [streamBandwidthTest, setStreamBandwidthTest] = useState(false);
  const [sceneCollection, setSceneCollection] = useState<SceneCollection>(() =>
    createDefaultSceneCollection(),
  );
  const [selectedSceneSourceId, setSelectedSceneSourceId] = useState(
    "source-main-display",
  );
  const [sceneSaveStatus, setSceneSaveStatus] = useState<string | null>(null);
  const [sceneDirty, setSceneDirty] = useState(false);

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
          nextRecentRecordings,
          nextMarkers,
          nextMediaRunnerInfo,
          nextPipelinePlan,
          nextSuiteStatus,
          nextSuiteSession,
          nextSuiteTimeline,
          nextTwitchReadiness,
        ] = await Promise.all([
          StudioApi.health(runtimeConfig),
          StudioApi.status(runtimeConfig),
          StudioApi.profiles(runtimeConfig),
          StudioApi.clients(runtimeConfig),
          StudioApi.auditLog(runtimeConfig),
          StudioApi.recentRecordings(runtimeConfig),
          StudioApi.markers(runtimeConfig, { limit: 20 }),
          loadMediaRunnerInfo(),
          StudioApi.mediaPlan(runtimeConfig),
          loadSuiteStatus(),
          loadSuiteSession(),
          loadSuiteTimeline(50),
          fetchTwitchBroadcastReadinessFromConsole(),
        ]);
        if (cancelled) return;
        setHealth(nextHealth);
        setStatus(nextStatus);
        setProfiles(nextProfiles);
        setClients(nextClients.clients);
        setAuditEntries(nextAuditLog.entries);
        setRecentRecordings(nextRecentRecordings.recordings);
        setRecentMarkers(nextMarkers.markers);
        setMediaRunnerInfo(nextMediaRunnerInfo);
        setPipelinePlan(nextPipelinePlan);
        setSuiteStatus(nextSuiteStatus);
        setSuiteSession(nextSuiteSession);
        setPersistedSuiteTimeline(nextSuiteTimeline);
        setTwitchReadiness(nextTwitchReadiness);
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
    let cancelled = false;
    StudioApi.sceneCollection(config)
      .then((collection) => {
        if (cancelled) return;
        setSceneCollection(collection);
        const scene =
          collection.scenes.find(
            (item) => item.id === collection.active_scene_id,
          ) ?? collection.scenes[0];
        setSelectedSceneSourceId(scene?.sources[0]?.id ?? "");
        setSceneDirty(false);
        setSceneSaveStatus(null);
      })
      .catch((error: Error) => {
        if (!cancelled) {
          setSceneSaveStatus(error.message);
        }
      });

    return () => {
      cancelled = true;
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
        recordStudioMediaEvent(event);
      }
      if (event.type === "recording.stopped") {
        StudioApi.recentRecordings(config)
          .then((snapshot) => setRecentRecordings(snapshot.recordings))
          .catch(() => undefined);
      }
      if (event.type === "marker.created") {
        StudioApi.markers(config, { limit: 20 })
          .then((snapshot) => setRecentMarkers(snapshot.markers))
          .catch(() => undefined);
      }
    };
    socket.onerror = () => setError("WebSocket event stream unavailable");

    return () => socket.close();
  }, [config]);

  const activeStatus = status?.status;
  const activeDestination = activeStatus?.active_destination;
  const recordingPath = activeStatus?.recording_path;
  const selectedDestination = profiles?.stream_destinations.find(
    (destination) => destination.id === selectedDestinationId,
  );
  const suiteTimeline = useMemo(
    () =>
      buildSuiteTimeline(
        suiteStatus,
        recentRecordings,
        recentMarkers,
        events,
        persistedSuiteTimeline,
    ),
    [events, persistedSuiteTimeline, recentMarkers, recentRecordings, suiteStatus],
  );
  const designerSceneCollection = useMemo(
    () => bindSceneCollectionCaptureInventory(sceneCollection, captureInventory),
    [captureInventory, sceneCollection],
  );
  const activeDesignerScene =
    designerSceneCollection.scenes.find(
      (scene) => scene.id === designerSceneCollection.active_scene_id,
    ) ?? designerSceneCollection.scenes[0];
  const activeCompositorGraph = useMemo(
    () => buildCompositorGraph(activeDesignerScene),
    [activeDesignerScene],
  );
  const selectedDesignerSource =
    activeDesignerScene?.sources.find(
      (source) => source.id === selectedSceneSourceId,
    ) ??
    activeDesignerScene?.sources[0] ??
    null;

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

  async function importTwitchStreamKeyFromConsole() {
    try {
      const imported = await fetchTwitchStreamKeyFromConsole();
      setDestinationForm((current) => ({
        ...current,
        name:
          current.name ||
          (imported.broadcasterLogin
            ? `Twitch - ${imported.broadcasterLogin}`
            : "Twitch Manual RTMP"),
        platform: "twitch",
        ingest_url: current.ingest_url || "rtmp://live.twitch.tv/app",
        stream_key: imported.streamKey,
        enabled: current.enabled ?? true,
      }));
      setError(null);
    } catch (error) {
      setError(
        error instanceof Error
          ? error.message
          : "Could not import the Twitch stream key from Console",
      );
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

  async function handleCaptureSourceToggle(
    candidate: CaptureSourceCandidate,
    enabled: boolean,
  ) {
    const nextSettings = updateSettingsCaptureSource(
      settingsSnapshot?.settings ?? settingsForm,
      candidate,
      enabled,
    );
    setSettingsForm(nextSettings);
    setCaptureSourceSaving(true);
    try {
      const snapshot = await saveAppSettings(nextSettings);
      applySettingsSnapshot(snapshot);
      await refreshCaptureContext();
      loadPreflightSnapshot().then(setPreflight).catch(() => undefined);
      setError(null);
    } catch (error) {
      setError(
        error instanceof Error ? error.message : "Capture source save failed",
      );
    } finally {
      setCaptureSourceSaving(false);
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

  async function handleLaunchSuite() {
    setSuiteLaunchStatus("Opening vaexcore apps...");
    const results = await launchVaexcoreSuite();
    const failed = results.filter((result) => !result.ok);

    if (failed.length > 0) {
      setSuiteLaunchStatus(formatSuiteLaunchFailure(failed));
      loadSuiteStatus().then(setSuiteStatus).catch(() => undefined);
      return;
    }

    setSuiteLaunchStatus("Launch requested. Verifying suite status...");
    window.setTimeout(() => {
      loadSuiteStatus()
        .then((status) => {
          setSuiteStatus(status);
          setSuiteLaunchStatus(formatSuiteVerification(status));
        })
        .catch(() => {
          setSuiteLaunchStatus("Launch requested for Studio, Pulse, and Console.");
        });
    }, 1800);
    setError(null);
  }

  async function handleStartSuiteSession() {
    try {
      const nextSession = await startSuiteSession(suiteSession?.title);
      setSuiteSession(nextSession);
      const status = await loadSuiteStatus();
      setSuiteStatus(status);
      setSuiteLaunchStatus(`Suite session active: ${nextSession.title}`);
      setError(null);
    } catch (error) {
      setError(
        error instanceof Error ? error.message : "Could not start suite session",
      );
    }
  }

  async function handleSendSuiteCommand(targetApp: string, command: string) {
    try {
      await sendSuiteCommand({
        targetApp,
        command,
        payload: {
          requestedFrom: "vaexcore-studio",
          requestedAt: new Date().toISOString(),
        },
      });
      setSuiteLaunchStatus(`${command} sent to ${targetApp}.`);
      setError(null);
    } catch (error) {
      setError(
        error instanceof Error ? error.message : "Could not send suite command",
      );
    }
  }

  async function handleReviewRecordingInPulse(recording: RecordingHistoryEntry) {
    setSuiteLaunchStatus(`Sending ${recording.profile_name} recording to Pulse...`);
    const results = await handoffRecordingToPulse({
      sessionId: recording.session_id,
      outputPath: recording.output_path,
      profileId: recording.profile_id,
      profileName: recording.profile_name,
      stoppedAt: recording.stopped_at,
    });
    const failed = results.filter((result) => !result.ok);

    if (failed.length > 0) {
      setSuiteLaunchStatus(formatSuiteLaunchFailure(failed));
      return;
    }

    setSuiteLaunchStatus("Pulse handoff written. Opening Pulse review workspace.");
    window.setTimeout(() => {
      loadSuiteStatus().then(setSuiteStatus).catch(() => undefined);
    }, 1800);
    setError(null);
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

  function handleSelectDesignerScene(sceneId: string) {
    const scene = sceneCollection.scenes.find((item) => item.id === sceneId);
    if (!scene) return;
    updateDesignerCollection((current) => ({
      ...current,
      active_scene_id: sceneId,
    }));
    setSelectedSceneSourceId(scene.sources[0]?.id ?? "");
  }

  function updateDesignerCollection(
    updater: (current: SceneCollection) => SceneCollection,
  ) {
    setSceneCollection((current) => ({
      ...updater(current),
      updated_at: new Date().toISOString(),
    }));
    setSceneDirty(true);
    setSceneSaveStatus("Unsaved scene changes");
  }

  function handleUpdateDesignerSource(
    sceneId: string,
    sourceId: string,
    patch: SceneSourcePatch,
  ) {
    updateDesignerCollection((current) => ({
      ...current,
      scenes: current.scenes.map((scene) =>
        scene.id === sceneId
          ? {
              ...scene,
              sources: scene.sources.map((source) =>
                source.id === sourceId
                  ? mergeSceneSourcePatch(source, patch)
                  : source,
              ),
            }
          : scene,
      ),
    }));
  }

  function handleReorderDesignerSource(
    sceneId: string,
    sourceId: string,
    direction: "up" | "down",
  ) {
    updateDesignerCollection((current) => ({
      ...current,
      scenes: current.scenes.map((scene) => {
        if (scene.id !== sceneId) return scene;
        const ordered = [...scene.sources].sort(
          (left, right) => left.z_index - right.z_index,
        );
        const index = ordered.findIndex((source) => source.id === sourceId);
        const swapIndex = direction === "up" ? index + 1 : index - 1;
        if (index < 0 || swapIndex < 0 || swapIndex >= ordered.length) {
          return scene;
        }

        const source = ordered[index];
        const swap = ordered[swapIndex];
        return {
          ...scene,
          sources: scene.sources.map((item) => {
            if (item.id === source.id) return { ...item, z_index: swap.z_index };
            if (item.id === swap.id) return { ...item, z_index: source.z_index };
            return item;
          }),
        };
      }),
    }));
  }

  function handleCreateDesignerScene() {
    const now = new Date().toISOString();
    const sceneId = designerId("scene");
    const source = createDefaultSceneSource("display", {
      id: designerId("source-display"),
      name: "Display Placeholder",
      position: { x: 0, y: 0 },
      size: { width: 1920, height: 1080 },
      z_index: 0,
    });
    updateDesignerCollection((current) => ({
      ...current,
      active_scene_id: sceneId,
      scenes: [
        ...current.scenes,
        {
          id: sceneId,
          name: `Scene ${current.scenes.length + 1}`,
          canvas: { width: 1920, height: 1080, background_color: "#050711" },
          sources: [source],
        },
      ],
      updated_at: now,
    }));
    setSelectedSceneSourceId(source.id);
  }

  function handleDuplicateDesignerScene(sceneId: string) {
    const scene = sceneCollection.scenes.find((item) => item.id === sceneId);
    if (!scene) return;
    const nextScene = {
      ...scene,
      id: designerId("scene"),
      name: `${scene.name} Copy`,
      sources: scene.sources.map((source) => ({
        ...source,
        id: designerId("source"),
      })) as SceneSource[],
    };
    updateDesignerCollection((current) => ({
      ...current,
      active_scene_id: nextScene.id,
      scenes: [...current.scenes, nextScene],
    }));
    setSelectedSceneSourceId(nextScene.sources[0]?.id ?? "");
  }

  function handleDeleteDesignerScene(sceneId: string) {
    if (sceneCollection.scenes.length <= 1) return;
    updateDesignerCollection((current) => {
      const scenes = current.scenes.filter((scene) => scene.id !== sceneId);
      const active_scene_id =
        current.active_scene_id === sceneId ? scenes[0].id : current.active_scene_id;
      return {
        ...current,
        active_scene_id,
        scenes,
      };
    });
    const nextScene = sceneCollection.scenes.find((scene) => scene.id !== sceneId);
    setSelectedSceneSourceId(nextScene?.sources[0]?.id ?? "");
  }

  function handleRenameDesignerScene(sceneId: string, name: string) {
    updateDesignerCollection((current) => ({
      ...current,
      scenes: current.scenes.map((scene) =>
        scene.id === sceneId ? { ...scene, name } : scene,
      ),
    }));
  }

  function handleCreateDesignerSource(sceneId: string, kind: SceneSourceKind) {
    const source = createDefaultSceneSource(kind, {
      id: designerId(`source-${kind}`),
      name: sceneSourceKindLabels[kind],
      position: { x: 120, y: 120 },
      size: defaultDesignerSourceSize(kind),
      z_index: nextSceneZIndex(activeDesignerScene),
    });
    updateDesignerCollection((current) => ({
      ...current,
      scenes: current.scenes.map((scene) =>
        scene.id === sceneId
          ? { ...scene, sources: [...scene.sources, source] }
          : scene,
      ),
    }));
    setSelectedSceneSourceId(source.id);
  }

  function handleDuplicateDesignerSource(sceneId: string, sourceId: string) {
    const source = activeDesignerScene.sources.find((item) => item.id === sourceId);
    if (!source) return;
    const duplicate = {
      ...source,
      id: designerId("source"),
      name: `${source.name} Copy`,
      position: {
        x: source.position.x + 32,
        y: source.position.y + 32,
      },
      z_index: nextSceneZIndex(activeDesignerScene),
    } as SceneSource;
    updateDesignerCollection((current) => ({
      ...current,
      scenes: current.scenes.map((scene) =>
        scene.id === sceneId
          ? { ...scene, sources: [...scene.sources, duplicate] }
          : scene,
      ),
    }));
    setSelectedSceneSourceId(duplicate.id);
  }

  function handleDeleteDesignerSource(sceneId: string, sourceId: string) {
    updateDesignerCollection((current) => ({
      ...current,
      scenes: current.scenes.map((scene) =>
        scene.id === sceneId
          ? {
              ...scene,
              sources: scene.sources.filter((source) => source.id !== sourceId),
            }
          : scene,
      ),
    }));
    const nextSource = activeDesignerScene.sources.find(
      (source) => source.id !== sourceId,
    );
    setSelectedSceneSourceId(nextSource?.id ?? "");
  }

  async function handleSaveDesignerScenes() {
    if (!config) return;
    try {
      const saved = await StudioApi.saveSceneCollection(config, sceneCollection);
      setSceneCollection(saved);
      const scene =
        saved.scenes.find((item) => item.id === saved.active_scene_id) ??
        saved.scenes[0];
      setSelectedSceneSourceId((current) =>
        scene.sources.some((source) => source.id === current)
          ? current
          : (scene.sources[0]?.id ?? ""),
      );
      setSceneDirty(false);
      setSceneSaveStatus("Scene collection saved");
      setError(null);
      StudioApi.mediaPlan(config).then(setPipelinePlan).catch(() => undefined);
    } catch (error) {
      setSceneSaveStatus(
        error instanceof Error ? error.message : "Scene save failed",
      );
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
      case "designer":
        return (
          <DesignerPage
            captureInventory={captureInventory}
            collection={designerSceneCollection}
            dirty={sceneDirty}
            onCreateScene={handleCreateDesignerScene}
            onCreateSource={handleCreateDesignerSource}
            onDeleteScene={handleDeleteDesignerScene}
            onDeleteSource={handleDeleteDesignerSource}
            onDuplicateScene={handleDuplicateDesignerScene}
            onDuplicateSource={handleDuplicateDesignerSource}
            onRenameScene={handleRenameDesignerScene}
            scene={activeDesignerScene}
            graph={activeCompositorGraph}
            saveStatus={sceneSaveStatus}
            selectedSource={selectedDesignerSource}
            selectedSourceId={selectedSceneSourceId}
            onReorderSource={handleReorderDesignerSource}
            onSave={handleSaveDesignerScenes}
            onSelectScene={handleSelectDesignerScene}
            onSelectSource={setSelectedSceneSourceId}
            onUpdateSource={handleUpdateDesignerSource}
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
            onImportTwitchKey={importTwitchStreamKeyFromConsole}
            onUseTwitchManual={() =>
              setDestinationForm({
                ...destinationForm,
                name: destinationForm.name || "Twitch Manual RTMP",
                platform: "twitch",
                ingest_url: "rtmp://live.twitch.tv/app",
                enabled: true,
              })
            }
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
            captureInventory={captureInventory}
            captureSourceSaving={captureSourceSaving}
            markerLabel={markerLabel}
            onMarkerLabelChange={setMarkerLabel}
            onCaptureSourceToggle={handleCaptureSourceToggle}
            onStartRecording={() =>
              config &&
              runCommand(() => StudioApi.startRecording(config, selectedProfileId))
            }
            onStartStream={() =>
              config &&
              runCommand(() =>
                StudioApi.startStream(
                  config,
                  selectedDestinationId,
                  streamBandwidthTest,
                ),
              )
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
            preflight={preflight}
            recordingActive={activeStatus?.recording_active ?? false}
            selectedDestinationId={selectedDestinationId}
            selectedDestination={selectedDestination}
            selectedProfileId={selectedProfileId}
            settings={settingsSnapshot?.settings ?? settingsForm}
            onOpenSettings={handleOpenSettingsWindow}
            setSelectedDestinationId={setSelectedDestinationId}
            setSelectedProfileId={setSelectedProfileId}
            streamBandwidthTest={streamBandwidthTest}
            setStreamBandwidthTest={setStreamBandwidthTest}
            twitchReadiness={twitchReadiness}
            engineMode={activeStatus?.mode ?? "dry_run"}
            mediaRunnerInfo={mediaRunnerInfo}
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
            onLaunchSuite={handleLaunchSuite}
            onStartSuiteSession={handleStartSuiteSession}
            onSendSuiteCommand={handleSendSuiteCommand}
            onReviewRecordingInPulse={handleReviewRecordingInPulse}
            recentMarkers={recentMarkers}
            recentRecordings={recentRecordings}
            suiteSession={suiteSession}
            suiteStatus={suiteStatus}
            suiteTimeline={suiteTimeline}
            suiteLaunchStatus={suiteLaunchStatus}
          />
        );
      case "logs":
        return <LogsPage auditEntries={auditEntries} events={events} />;
    }
  }, [
    activeDestination?.name,
    activeStatus?.engine,
    activeStatus?.mode,
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
    recentMarkers,
    recentRecordings,
    recordingPath,
    section,
    selectedDestinationId,
    selectedProfileId,
    selectedDestination,
    suiteStatus,
    suiteSession,
    suiteTimeline,
    suiteLaunchStatus,
    streamBandwidthTest,
    twitchReadiness,
    activeDesignerScene,
    captureInventory,
    designerSceneCollection,
    sceneCollection,
    sceneDirty,
    sceneSaveStatus,
    selectedDesignerSource,
    selectedSceneSourceId,
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
            <span>control room</span>
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
            <button
              className="secondary-button compact"
              onClick={handleLaunchSuite}
              type="button"
            >
              <Play size={14} />
              Launch & Verify
            </button>
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

function DesignerPage(props: {
  captureInventory: CaptureSourceInventory | null;
  collection: SceneCollection;
  dirty: boolean;
  onCreateScene: () => void;
  onCreateSource: (sceneId: string, kind: SceneSourceKind) => void;
  onDeleteScene: (sceneId: string) => void;
  onDeleteSource: (sceneId: string, sourceId: string) => void;
  onDuplicateScene: (sceneId: string) => void;
  onDuplicateSource: (sceneId: string, sourceId: string) => void;
  onRenameScene: (sceneId: string, name: string) => void;
  graph: CompositorGraph;
  scene: Scene;
  saveStatus: string | null;
  selectedSource: SceneSource | null;
  selectedSourceId: string;
  onReorderSource: (
    sceneId: string,
    sourceId: string,
    direction: "up" | "down",
  ) => void;
  onSelectScene: (sceneId: string) => void;
  onSelectSource: (sourceId: string) => void;
  onSave: () => void;
  onUpdateSource: (
    sceneId: string,
    sourceId: string,
    patch: SceneSourcePatch,
  ) => void;
}) {
  const validation = validateSceneCollection(props.collection);
  const graphValidation = validateCompositorGraph(props.graph);
  const sourceStack = sortedSceneSources(props.scene);
  const canvasRef = useRef<HTMLDivElement | null>(null);
  const renderCanvasRef = useRef<HTMLCanvasElement | null>(null);
  const [dragState, setDragState] = useState<DesignerDragState | null>(null);
  const [newSourceKind, setNewSourceKind] = useState<SceneSourceKind>("display");

  useEffect(() => {
    drawCompositorPreview(
      renderCanvasRef.current,
      props.graph,
      props.selectedSourceId,
    );
  }, [props.graph, props.selectedSourceId]);

  function beginSourceDrag(
    event: ReactPointerEvent,
    source: SceneSource,
    mode: "move" | "resize",
  ) {
    if (source.locked || event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    props.onSelectSource(source.id);
    canvasRef.current?.setPointerCapture(event.pointerId);
    setDragState({
      mode,
      pointerId: event.pointerId,
      sourceId: source.id,
      startClientX: event.clientX,
      startClientY: event.clientY,
      startPosition: { ...source.position },
      startSize: { ...source.size },
    });
  }

  function updateDrag(event: ReactPointerEvent) {
    if (!dragState || dragState.pointerId !== event.pointerId) return;
    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;
    const deltaX = ((event.clientX - dragState.startClientX) / rect.width) * props.scene.canvas.width;
    const deltaY = ((event.clientY - dragState.startClientY) / rect.height) * props.scene.canvas.height;

    if (dragState.mode === "move") {
      props.onUpdateSource(props.scene.id, dragState.sourceId, {
        position: {
          x: Math.round(dragState.startPosition.x + deltaX),
          y: Math.round(dragState.startPosition.y + deltaY),
        },
      });
      return;
    }

    props.onUpdateSource(props.scene.id, dragState.sourceId, {
      size: {
        width: Math.max(16, Math.round(dragState.startSize.width + deltaX)),
        height: Math.max(16, Math.round(dragState.startSize.height + deltaY)),
      },
    });
  }

  function endDrag(event: ReactPointerEvent) {
    if (dragState?.pointerId === event.pointerId) {
      setDragState(null);
    }
  }

  return (
    <div className="designer-grid">
      <div className="designer-left-rail">
        <section className="panel">
          <PanelTitle
            action={
              <button
                className="secondary-button compact"
                onClick={props.onCreateScene}
                type="button"
              >
                <Plus size={14} />
                Scene
              </button>
            }
            title="Scenes"
          />
          <div className="designer-list">
            {props.collection.scenes.map((scene) => (
              <div
                className={
                  scene.id === props.scene.id
                    ? "designer-list-item selected"
                    : "designer-list-item"
                }
                key={scene.id}
              >
                <button
                  className="source-stack-select-button"
                  onClick={() => props.onSelectScene(scene.id)}
                  type="button"
                >
                  <div>
                    <strong>{scene.name}</strong>
                    <span>
                      {scene.canvas.width}x{scene.canvas.height} - {scene.sources.length} sources
                    </span>
                  </div>
                </button>
                <Pill tone={scene.id === props.collection.active_scene_id ? "green" : "muted"}>
                  {scene.id === props.collection.active_scene_id ? "Active" : "Ready"}
                </Pill>
                <div className="source-stack-actions">
                  <button
                    aria-label={`Duplicate ${scene.name}`}
                    className="icon-button compact"
                    onClick={() => props.onDuplicateScene(scene.id)}
                    title={`Duplicate ${scene.name}`}
                    type="button"
                  >
                    <Copy size={14} />
                  </button>
                  <button
                    aria-label={`Delete ${scene.name}`}
                    className="icon-button compact danger"
                    disabled={props.collection.scenes.length <= 1}
                    onClick={() => props.onDeleteScene(scene.id)}
                    title={`Delete ${scene.name}`}
                    type="button"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>

        <section className="panel">
          <PanelTitle
            action={
              <div className="source-add-controls">
                <select
                  aria-label="New source kind"
                  value={newSourceKind}
                  onChange={(event) =>
                    setNewSourceKind(event.target.value as SceneSourceKind)
                  }
                >
                  {Object.entries(sceneSourceKindLabels).map(([kind, label]) => (
                    <option key={kind} value={kind}>
                      {label}
                    </option>
                  ))}
                </select>
                <button
                  className="secondary-button compact"
                  onClick={() => props.onCreateSource(props.scene.id, newSourceKind)}
                  type="button"
                >
                  <Plus size={14} />
                  Source
                </button>
              </div>
            }
            title="Source Stack"
          />
          <div className="designer-list source-stack">
            {sourceStack.map((source, index) => (
              <div
                className={
                  source.id === props.selectedSourceId
                    ? "designer-list-item source-stack-item selected"
                    : "designer-list-item source-stack-item"
                }
                key={source.id}
              >
                <button
                  className="source-stack-select-button"
                  onClick={() => props.onSelectSource(source.id)}
                  type="button"
                >
                  <div className="source-stack-main">
                    <SourceKindIcon kind={source.kind} />
                    <div>
                      <strong>{source.name}</strong>
                      <span>
                        {sceneSourceKindLabels[source.kind]} - z {source.z_index}
                      </span>
                    </div>
                  </div>
                </button>
                <div className="source-stack-actions">
                  <button
                    aria-label={`${source.visible ? "Hide" : "Show"} ${source.name}`}
                    className="icon-button compact"
                    onClick={() => {
                      props.onUpdateSource(props.scene.id, source.id, {
                        visible: !source.visible,
                      });
                    }}
                    title={`${source.visible ? "Hide" : "Show"} ${source.name}`}
                    type="button"
                  >
                    {source.visible ? <Eye size={14} /> : <EyeOff size={14} />}
                  </button>
                  <button
                    aria-label={`${source.locked ? "Unlock" : "Lock"} ${source.name}`}
                    className="icon-button compact"
                    onClick={() => {
                      props.onUpdateSource(props.scene.id, source.id, {
                        locked: !source.locked,
                      });
                    }}
                    title={`${source.locked ? "Unlock" : "Lock"} ${source.name}`}
                    type="button"
                  >
                    {source.locked ? <Lock size={14} /> : <Unlock size={14} />}
                  </button>
                  <button
                    aria-label={`Move ${source.name} forward`}
                    className="icon-button compact"
                    disabled={index === 0}
                    onClick={() => {
                      props.onReorderSource(props.scene.id, source.id, "up");
                    }}
                    title={`Move ${source.name} forward`}
                    type="button"
                  >
                    <ArrowUp size={14} />
                  </button>
                  <button
                    aria-label={`Move ${source.name} backward`}
                    className="icon-button compact"
                    disabled={index === sourceStack.length - 1}
                    onClick={() => {
                      props.onReorderSource(props.scene.id, source.id, "down");
                    }}
                    title={`Move ${source.name} backward`}
                    type="button"
                  >
                    <ArrowDown size={14} />
                  </button>
                  <button
                    aria-label={`Duplicate ${source.name}`}
                    className="icon-button compact"
                    onClick={() =>
                      props.onDuplicateSource(props.scene.id, source.id)
                    }
                    title={`Duplicate ${source.name}`}
                    type="button"
                  >
                    <Copy size={14} />
                  </button>
                  <button
                    aria-label={`Delete ${source.name}`}
                    className="icon-button compact danger"
                    onClick={() => props.onDeleteSource(props.scene.id, source.id)}
                    title={`Delete ${source.name}`}
                    type="button"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        </section>
      </div>

      <section className="panel designer-preview-panel">
        <PanelTitle
          action={
            <div className="button-row">
              <Pill tone={validation.ok ? "green" : "amber"}>
                {validation.ok ? "Valid" : `${validation.issues.length} issues`}
              </Pill>
              <Pill tone={graphValidation.ready ? "green" : "red"}>
                {graphValidation.ready
                  ? `${props.graph.nodes.length} graph nodes`
                  : "Graph blocked"}
              </Pill>
              {props.saveStatus && (
                <Pill tone={props.dirty ? "amber" : "green"}>
                  {props.saveStatus}
                </Pill>
              )}
              <button
                className="secondary-button compact"
                disabled={!validation.ok || !props.dirty}
                onClick={props.onSave}
                type="button"
              >
                <CheckCircle2 size={14} />
                Save
              </button>
            </div>
          }
          title="Preview"
        />
        <div className="designer-preview-shell">
          <div
            className="designer-preview-canvas"
            onPointerMove={updateDrag}
            onPointerUp={endDrag}
            onPointerCancel={endDrag}
            ref={canvasRef}
            style={{
              aspectRatio: `${props.scene.canvas.width} / ${props.scene.canvas.height}`,
              backgroundColor: props.scene.canvas.background_color,
            }}
          >
            <canvas
              aria-label="Compositor preview render"
              className="designer-preview-render-canvas"
              height={props.graph.output.height}
              ref={renderCanvasRef}
              width={props.graph.output.width}
            />
            {sortedSceneSources(props.scene, "asc").map((source) => (
              <div
                className={[
                  "designer-source-box",
                  `source-${source.kind}`,
                  source.id === props.selectedSourceId ? "selected" : "",
                  source.visible ? "" : "hidden-source",
                  source.locked ? "locked-source" : "",
                ]
                  .filter(Boolean)
                  .join(" ")}
                key={source.id}
                onClick={() => props.onSelectSource(source.id)}
                onKeyDown={(event) => {
                  if (source.locked) return;
                  const nudge = event.shiftKey ? 10 : 1;
                  if (event.key === "ArrowLeft") {
                    props.onUpdateSource(props.scene.id, source.id, {
                      position: { x: source.position.x - nudge },
                    });
                  } else if (event.key === "ArrowRight") {
                    props.onUpdateSource(props.scene.id, source.id, {
                      position: { x: source.position.x + nudge },
                    });
                  } else if (event.key === "ArrowUp") {
                    props.onUpdateSource(props.scene.id, source.id, {
                      position: { y: source.position.y - nudge },
                    });
                  } else if (event.key === "ArrowDown") {
                    props.onUpdateSource(props.scene.id, source.id, {
                      position: { y: source.position.y + nudge },
                    });
                  } else {
                    return;
                  }
                  event.preventDefault();
                }}
                onPointerDown={(event) => beginSourceDrag(event, source, "move")}
                role="button"
                style={sceneSourcePreviewStyle(source, props.scene)}
                tabIndex={0}
              >
                <div className="designer-source-label">
                  <SourceKindIcon kind={source.kind} />
                  <strong>{source.name}</strong>
                </div>
                <span>{sourceConfigSummary(source)}</span>
                {sceneSourceAvailability(source) && (
                  <small>{sceneSourceAvailability(source)?.detail}</small>
                )}
                {!source.locked && (
                  <button
                    aria-label={`Resize ${source.name}`}
                    className="designer-resize-handle"
                    onClick={(event) => event.stopPropagation()}
                    onPointerDown={(event) =>
                      beginSourceDrag(event, source, "resize")
                    }
                    title={`Resize ${source.name}`}
                    type="button"
                  />
                )}
              </div>
            ))}
          </div>
        </div>
        <div className="designer-preview-meta">
          <KeyValue
            label="Canvas"
            value={`${props.scene.canvas.width}x${props.scene.canvas.height}`}
          />
          <KeyValue label="Collection" value={props.collection.name} />
          <KeyValue label="Updated" value={formatSuiteTimestamp(props.collection.updated_at)} />
          <KeyValue
            label="Graph"
            value={`${props.graph.nodes.filter((node) => node.visible).length}/${props.graph.nodes.length} visible`}
          />
          <KeyValue
            label="Graph Status"
            value={
              graphValidation.ready
                ? graphValidation.warnings.length
                  ? `${graphValidation.warnings.length} warnings`
                  : "ready"
                : `${graphValidation.errors.length} errors`
            }
          />
        </div>
      </section>

      <section className="panel designer-inspector">
        <PanelTitle title="Inspector" />
        {props.selectedSource ? (
          <div className="form">
            <TextInput
              label="Scene Name"
              onChange={(name) => props.onRenameScene(props.scene.id, name)}
              value={props.scene.name}
            />
            <TextInput
              label="Name"
              onChange={(name) =>
                props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                  name,
                })
              }
              value={props.selectedSource.name}
            />
            <div className="form-grid">
              <SceneNumberInput
                label="X"
                onChange={(x) =>
                  props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                    position: { x },
                  })
                }
                value={props.selectedSource.position.x}
              />
              <SceneNumberInput
                label="Y"
                onChange={(y) =>
                  props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                    position: { y },
                  })
                }
                value={props.selectedSource.position.y}
              />
            </div>
            <div className="form-grid">
              <SceneNumberInput
                label="Width"
                min={1}
                onChange={(width) =>
                  props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                    size: { width },
                  })
                }
                value={props.selectedSource.size.width}
              />
              <SceneNumberInput
                label="Height"
                min={1}
                onChange={(height) =>
                  props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                    size: { height },
                  })
                }
                value={props.selectedSource.size.height}
              />
            </div>
            <div className="form-grid">
              <SceneNumberInput
                label="Crop Top"
                min={0}
                onChange={(top) =>
                  props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                    crop: { top },
                  })
                }
                value={props.selectedSource.crop.top}
              />
              <SceneNumberInput
                label="Crop Right"
                min={0}
                onChange={(right) =>
                  props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                    crop: { right },
                  })
                }
                value={props.selectedSource.crop.right}
              />
            </div>
            <div className="form-grid">
              <SceneNumberInput
                label="Crop Bottom"
                min={0}
                onChange={(bottom) =>
                  props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                    crop: { bottom },
                  })
                }
                value={props.selectedSource.crop.bottom}
              />
              <SceneNumberInput
                label="Crop Left"
                min={0}
                onChange={(left) =>
                  props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                    crop: { left },
                  })
                }
                value={props.selectedSource.crop.left}
              />
            </div>
            <div className="form-grid">
              <SceneNumberInput
                label="Rotation"
                onChange={(rotation_degrees) =>
                  props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                    rotation_degrees,
                  })
                }
                step={1}
                value={props.selectedSource.rotation_degrees}
              />
              <SceneNumberInput
                label="Opacity"
                max={1}
                min={0}
                onChange={(opacity) =>
                  props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                    opacity,
                  })
                }
                step={0.05}
                value={props.selectedSource.opacity}
              />
            </div>
            <SceneNumberInput
              label="Z Index"
              onChange={(z_index) =>
                props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                  z_index,
                })
              }
              step={1}
              value={props.selectedSource.z_index}
            />
            <label className="check-row">
              <input
                checked={props.selectedSource.visible}
                onChange={(event) =>
                  props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                    visible: event.target.checked,
                  })
                }
                type="checkbox"
              />
              Visible
            </label>
            <label className="check-row">
              <input
                checked={props.selectedSource.locked}
                onChange={(event) =>
                  props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                    locked: event.target.checked,
                  })
                }
                type="checkbox"
              />
              Locked
            </label>
            <KeyValue
              label="Source Kind"
              value={sceneSourceKindLabels[props.selectedSource.kind]}
            />
            <SourceConfigEditor
              captureInventory={props.captureInventory}
              onChange={(config) =>
                props.onUpdateSource(props.scene.id, props.selectedSource!.id, {
                  config,
                })
              }
              source={props.selectedSource}
            />
            {sceneSourceAvailability(props.selectedSource) && (
              <KeyValue
                label="Availability"
                value={`${sceneSourceAvailability(props.selectedSource)?.state}: ${sceneSourceAvailability(props.selectedSource)?.detail}`}
              />
            )}
          </div>
        ) : (
          <div className="empty">No source selected</div>
        )}
      </section>
    </div>
  );
}

function SourceConfigEditor(props: {
  captureInventory: CaptureSourceInventory | null;
  onChange: (config: Record<string, unknown>) => void;
  source: SceneSource;
}) {
  const candidates = props.captureInventory?.candidates ?? [];
  const source = props.source;

  switch (source.kind) {
    case "display":
      return (
        <div className="source-config-editor">
          <label>
            Display
            <select
              value={source.config.display_id ?? ""}
              onChange={(event) =>
                props.onChange({ display_id: event.target.value || null })
              }
            >
              <option value="">Unassigned display</option>
              {candidates
                .filter((candidate) => candidate.kind === "display")
                .map((candidate) => (
                  <option key={candidate.id} value={candidate.id}>
                    {candidate.name}
                  </option>
                ))}
            </select>
          </label>
          <label className="check-row">
            <input
              checked={source.config.capture_cursor}
              onChange={(event) =>
                props.onChange({ capture_cursor: event.target.checked })
              }
              type="checkbox"
            />
            Capture cursor
          </label>
        </div>
      );
    case "window":
      return (
        <div className="source-config-editor">
          <label>
            Window
            <select
              value={source.config.window_id ?? ""}
              onChange={(event) =>
                props.onChange({ window_id: event.target.value || null })
              }
            >
              <option value="">Unassigned window</option>
              {candidates
                .filter((candidate) => candidate.kind === "window")
                .map((candidate) => (
                  <option key={candidate.id} value={candidate.id}>
                    {candidate.name}
                  </option>
                ))}
            </select>
          </label>
          <TextInput
            label="Application"
            onChange={(application_name) => props.onChange({ application_name })}
            value={source.config.application_name ?? ""}
          />
          <TextInput
            label="Title"
            onChange={(title) => props.onChange({ title })}
            value={source.config.title ?? ""}
          />
        </div>
      );
    case "camera":
      return (
        <div className="source-config-editor">
          <label>
            Camera
            <select
              value={source.config.device_id ?? ""}
              onChange={(event) =>
                props.onChange({ device_id: event.target.value || null })
              }
            >
              <option value="">Unassigned camera</option>
              {candidates
                .filter((candidate) => candidate.kind === "camera")
                .map((candidate) => (
                  <option key={candidate.id} value={candidate.id}>
                    {candidate.name}
                  </option>
                ))}
            </select>
          </label>
          <div className="form-grid">
            <SceneNumberInput
              label="Width"
              min={1}
              onChange={(width) =>
                props.onChange({
                  resolution: {
                    ...(source.config.resolution ?? {
                      width: 1280,
                      height: 720,
                    }),
                    width,
                  },
                })
              }
              value={source.config.resolution?.width ?? 1280}
            />
            <SceneNumberInput
              label="Height"
              min={1}
              onChange={(height) =>
                props.onChange({
                  resolution: {
                    ...(source.config.resolution ?? {
                      width: 1280,
                      height: 720,
                    }),
                    height,
                  },
                })
              }
              value={source.config.resolution?.height ?? 720}
            />
          </div>
          <SceneNumberInput
            label="Framerate"
            min={1}
            onChange={(framerate) => props.onChange({ framerate })}
            value={source.config.framerate ?? 30}
          />
        </div>
      );
    case "audio_meter":
      return (
        <div className="source-config-editor">
          <label>
            Audio Device
            <select
              value={source.config.device_id ?? ""}
              onChange={(event) =>
                props.onChange({ device_id: event.target.value || null })
              }
            >
              <option value="">Unassigned audio device</option>
              {candidates
                .filter(
                  (candidate) =>
                    candidate.kind === "microphone" ||
                    candidate.kind === "system_audio",
                )
                .map((candidate) => (
                  <option key={candidate.id} value={candidate.id}>
                    {candidate.name}
                  </option>
                ))}
            </select>
          </label>
          <div className="form-grid">
            <label>
              Channel
              <select
                value={source.config.channel}
                onChange={(event) =>
                  props.onChange({
                    channel: event.target.value as "microphone" | "system" | "mixed",
                  })
                }
              >
                <option value="microphone">Microphone</option>
                <option value="system">System</option>
                <option value="mixed">Mixed</option>
              </select>
            </label>
            <label>
              Meter
              <select
                value={source.config.meter_style}
                onChange={(event) =>
                  props.onChange({
                    meter_style: event.target.value as "bar" | "waveform",
                  })
                }
              >
                <option value="bar">Bar</option>
                <option value="waveform">Waveform</option>
              </select>
            </label>
          </div>
        </div>
      );
    case "image_media":
      return (
        <div className="source-config-editor">
          <TextInput
            label="Asset URI"
            onChange={(asset_uri) => props.onChange({ asset_uri })}
            value={source.config.asset_uri ?? ""}
          />
          <div className="form-grid">
            <label>
              Media Type
              <select
                value={source.config.media_type}
                onChange={(event) =>
                  props.onChange({
                    media_type: event.target.value as "image" | "video",
                  })
                }
              >
                <option value="image">Image</option>
                <option value="video">Video</option>
              </select>
            </label>
            <label className="check-row">
              <input
                checked={source.config.loop}
                onChange={(event) => props.onChange({ loop: event.target.checked })}
                type="checkbox"
              />
              Loop
            </label>
          </div>
        </div>
      );
    case "browser_overlay":
      return (
        <div className="source-config-editor">
          <TextInput
            label="URL"
            onChange={(url) => props.onChange({ url })}
            value={source.config.url ?? ""}
          />
          <div className="form-grid">
            <SceneNumberInput
              label="Viewport Width"
              min={1}
              onChange={(width) =>
                props.onChange({
                  viewport: { ...source.config.viewport, width },
                })
              }
              value={source.config.viewport.width}
            />
            <SceneNumberInput
              label="Viewport Height"
              min={1}
              onChange={(height) =>
                props.onChange({
                  viewport: { ...source.config.viewport, height },
                })
              }
              value={source.config.viewport.height}
            />
          </div>
        </div>
      );
    case "text":
      return (
        <div className="source-config-editor">
          <TextInput
            label="Text"
            onChange={(text) => props.onChange({ text })}
            value={source.config.text}
          />
          <div className="form-grid">
            <TextInput
              label="Font"
              onChange={(font_family) => props.onChange({ font_family })}
              value={source.config.font_family}
            />
            <SceneNumberInput
              label="Font Size"
              min={1}
              onChange={(font_size) => props.onChange({ font_size })}
              value={source.config.font_size}
            />
          </div>
          <div className="form-grid">
            <TextInput
              label="Color"
              onChange={(color) => props.onChange({ color })}
              value={source.config.color}
            />
            <label>
              Align
              <select
                value={source.config.align}
                onChange={(event) =>
                  props.onChange({
                    align: event.target.value as "left" | "center" | "right",
                  })
                }
              >
                <option value="left">Left</option>
                <option value="center">Center</option>
                <option value="right">Right</option>
              </select>
            </label>
          </div>
        </div>
      );
    case "group":
      return (
        <div className="source-config-editor">
          <TextInput
            label="Child Source IDs"
            onChange={(value) =>
              props.onChange({
                child_source_ids: value
                  .split(",")
                  .map((item) => item.trim())
                  .filter(Boolean),
              })
            }
            value={source.config.child_source_ids.join(", ")}
          />
        </div>
      );
  }
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
  onImportTwitchKey: () => void;
  onUseTwitchManual: () => void;
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
        <button
          className="secondary-button full"
          onClick={props.onUseTwitchManual}
          type="button"
        >
          <Radio size={16} />
          Twitch Manual RTMP
        </button>
        <button
          className="secondary-button full"
          onClick={props.onImportTwitchKey}
          type="button"
        >
          <Link2 size={16} />
          Import Twitch Key from Console
        </button>
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
  captureInventory: CaptureSourceInventory | null;
  captureSourceSaving: boolean;
  markerLabel: string;
  onCaptureSourceToggle: (
    candidate: CaptureSourceCandidate,
    enabled: boolean,
  ) => void;
  onCreateMarker: () => void;
  onMarkerLabelChange: (value: string) => void;
  onStartRecording: () => void;
  onStartStream: () => void;
  onStopRecording: () => void;
  onStopStream: () => void;
  profiles: ProfilesSnapshot | null;
  preflight: PreflightSnapshot | null;
  recordingActive: boolean;
  selectedDestinationId: string | undefined;
  selectedDestination: StreamDestination | undefined;
  selectedProfileId: string | undefined;
  settings: AppSettings | null;
  onOpenSettings: () => void;
  setSelectedDestinationId: (value: string) => void;
  setSelectedProfileId: (value: string) => void;
  streamBandwidthTest: boolean;
  setStreamBandwidthTest: (value: boolean) => void;
  twitchReadiness: TwitchBroadcastReadiness | null;
  engineMode: string;
  mediaRunnerInfo: MediaRunnerInfo | null;
  streamActive: boolean;
}) {
  const checklist = goLiveChecklist(
    props.selectedDestination,
    props.engineMode,
    props.mediaRunnerInfo,
    props.preflight,
    props.twitchReadiness,
  );
  const enabledSources =
    props.settings?.capture_sources.filter((source) => source.enabled) ?? [];
  const defaultProfile = props.settings?.default_recording_profile;
  const sourceCandidates =
    props.captureInventory?.candidates ??
    props.settings?.capture_sources.map((source) => ({
      id: source.id,
      kind: source.kind,
      name: source.name,
      available: true,
      notes: null,
    })) ??
    [];
  const recordingButtonLabel =
    props.engineMode === "dry_run" || props.mediaRunnerInfo?.fallbackDryRun
      ? "Start Dry-Run Recording"
      : "Start Recording";

  return (
    <div className="control-grid">
      <section className="panel">
        <PanelTitle title="Capture Sources" />
        {sourceCandidates.length === 0 ? (
          <div className="empty">No capture sources found</div>
        ) : (
          <div className="source-grid">
            {sourceCandidates.map((candidate) => {
              const selected = props.settings?.capture_sources.find(
                (source) => source.id === candidate.id,
              );
              const checked = selected?.enabled ?? false;
              return (
                <label className="source-option" key={candidate.id}>
                  <input
                    checked={checked}
                    disabled={!candidate.available || props.captureSourceSaving}
                    onChange={(event) =>
                      props.onCaptureSourceToggle(
                        candidate,
                        event.target.checked,
                      )
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
        )}
        {props.captureSourceSaving && <Pill tone="amber">Saving</Pill>}
      </section>

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
        <label className="check-row">
          <input
            checked={props.streamBandwidthTest}
            disabled={props.streamActive}
            onChange={(event) => props.setStreamBandwidthTest(event.target.checked)}
            type="checkbox"
          />
          Twitch bandwidth test
        </label>
        <div className="button-row">
          <button
            className="primary-button danger"
            disabled={props.recordingActive || enabledSources.length === 0}
            onClick={props.onStartRecording}
            type="button"
          >
            <Play size={16} />
            {recordingButtonLabel}
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
        <PanelTitle title="Broadcast Setup" />
        <div className="checklist">
          <div className="checklist-row">
            <Pill tone={enabledSources.length ? "green" : "amber"}>
              {enabledSources.length ? "Ready" : "Check"}
            </Pill>
            <div>
              <strong>Capture</strong>
              <span>
                {enabledSources.length
                  ? enabledSources.map((source) => source.name).join(", ")
                  : "Enable a display, window, or camera source."}
              </span>
            </div>
          </div>
          <div className="checklist-row">
            <Pill tone={defaultProfile ? "green" : "amber"}>
              {defaultProfile ? "Ready" : "Check"}
            </Pill>
            <div>
              <strong>Quality</strong>
              <span>
                {defaultProfile
                  ? `${defaultProfile.resolution.width}x${defaultProfile.resolution.height} at ${defaultProfile.framerate} fps, ${defaultProfile.bitrate_kbps} kbps`
                  : "Select a default recording profile."}
              </span>
            </div>
          </div>
        </div>
        <button
          className="secondary-button full"
          onClick={props.onOpenSettings}
          type="button"
        >
          <SlidersHorizontal size={16} />
          Open Capture Settings
        </button>
      </section>

      <section className="panel">
        <PanelTitle title="Go Live Checklist" />
        <div className="checklist">
          {checklist.map((item) => (
            <div className="checklist-row" key={item.label}>
              <Pill tone={item.ready ? "green" : "amber"}>
                {item.ready ? "Ready" : "Check"}
              </Pill>
              <div>
                <strong>{item.label}</strong>
                <span>{item.detail}</span>
              </div>
            </div>
          ))}
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

function goLiveChecklist(
  destination: StreamDestination | undefined,
  engineMode: string,
  runner: MediaRunnerInfo | null,
  preflight: PreflightSnapshot | null,
  twitchReadiness: TwitchBroadcastReadiness | null,
) {
  const isTwitch = destination?.platform === "twitch";
  const ingestReady = Boolean(destination?.ingest_url?.trim());
  const keyReady = Boolean(destination?.stream_key_ref);
  const runnerReady = Boolean(runner?.running) && engineMode !== "dry_run";
  const preflightReady =
    !preflight || preflight.overall === "ready" || preflight.overall === "warning";

  return [
    {
      label: "Destination",
      ready: Boolean(destination?.enabled) && ingestReady,
      detail: destination
        ? `${platformLabels[destination.platform]} at ${destination.ingest_url || "no ingest URL"}`
        : "Select an enabled stream destination.",
    },
    {
      label: "Stream Key",
      ready: keyReady || !isTwitch,
      detail: keyReady
        ? "A local stream key is stored."
        : "Store a Twitch stream key before going live.",
    },
    {
      label: "Twitch Readiness",
      ready: !isTwitch || twitchReadiness?.ok === true,
      detail: !isTwitch
        ? "This destination does not require Twitch validation."
        : twitchReadiness
          ? twitchReadiness.summary
          : "Console has not published Twitch broadcast readiness yet.",
    },
    {
      label: "Media Runner",
      ready: runnerReady,
      detail: runnerReady
        ? "Real RTMP runner is available."
        : "Studio is still using dry-run media.",
    },
    {
      label: "Permissions",
      ready: preflightReady,
      detail: preflight
        ? `Preflight status is ${preflight.overall}.`
        : "Preflight status has not loaded yet.",
    },
  ];
}

function mergeSceneSourcePatch(
  source: SceneSource,
  patch: SceneSourcePatch,
): SceneSource {
  return {
    ...source,
    ...patch,
    position: patch.position
      ? { ...source.position, ...patch.position }
      : source.position,
    size: patch.size ? { ...source.size, ...patch.size } : source.size,
    crop: patch.crop ? { ...source.crop, ...patch.crop } : source.crop,
    config: patch.config ? { ...source.config, ...patch.config } : source.config,
  } as SceneSource;
}

function defaultDesignerSourceSize(kind: SceneSourceKind): SceneSize {
  switch (kind) {
    case "display":
    case "window":
      return { width: 1280, height: 720 };
    case "camera":
      return { width: 380, height: 214 };
    case "audio_meter":
      return { width: 420, height: 72 };
    case "image_media":
      return { width: 640, height: 360 };
    case "browser_overlay":
      return { width: 560, height: 170 };
    case "text":
      return { width: 640, height: 110 };
    case "group":
      return { width: 720, height: 420 };
  }
}

function nextSceneZIndex(scene: Scene): number {
  return scene.sources.reduce((highest, source) => Math.max(highest, source.z_index), 0) + 10;
}

function designerId(prefix: string): string {
  const random =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID()
      : Math.random().toString(36).slice(2);
  return `${prefix}-${random}`;
}

function sortedSceneSources(
  scene: Scene,
  direction: "asc" | "desc" = "desc",
): SceneSource[] {
  const multiplier = direction === "asc" ? 1 : -1;
  return [...scene.sources].sort(
    (left, right) => (left.z_index - right.z_index) * multiplier,
  );
}

function drawCompositorPreview(
  canvas: HTMLCanvasElement | null,
  graph: CompositorGraph,
  selectedSourceId: string,
) {
  if (!canvas) return;
  const context = canvas.getContext("2d");
  if (!context) return;

  canvas.width = Math.max(1, graph.output.width);
  canvas.height = Math.max(1, graph.output.height);
  context.clearRect(0, 0, canvas.width, canvas.height);
  context.fillStyle = graph.output.background_color || "#050711";
  context.fillRect(0, 0, canvas.width, canvas.height);

  context.save();
  context.strokeStyle = "rgba(255, 255, 255, 0.06)";
  context.lineWidth = 1;
  const gridSize = Math.max(80, Math.round(graph.output.width / 24));
  for (let x = gridSize; x < graph.output.width; x += gridSize) {
    context.beginPath();
    context.moveTo(x, 0);
    context.lineTo(x, graph.output.height);
    context.stroke();
  }
  for (let y = gridSize; y < graph.output.height; y += gridSize) {
    context.beginPath();
    context.moveTo(0, y);
    context.lineTo(graph.output.width, y);
    context.stroke();
  }
  context.restore();

  graph.nodes
    .filter((node) => node.visible)
    .forEach((node) => drawCompositorNode(context, graph, node, selectedSourceId));
}

function drawCompositorNode(
  context: CanvasRenderingContext2D,
  graph: CompositorGraph,
  node: CompositorNode,
  selectedSourceId: string,
) {
  const width = Math.max(1, node.transform.size.width);
  const height = Math.max(1, node.transform.size.height);
  const x = node.transform.position.x;
  const y = node.transform.position.y;
  const selected = node.source_id === selectedSourceId;

  context.save();
  context.translate(x + width / 2, y + height / 2);
  context.rotate((node.transform.rotation_degrees * Math.PI) / 180);
  context.globalAlpha = clamp(node.transform.opacity, 0, 1);
  context.fillStyle = compositorNodeFill(node);
  context.strokeStyle = selected ? "#39d9ff" : compositorNodeStroke(node);
  context.lineWidth = selected ? 6 : 3;
  context.fillRect(-width / 2, -height / 2, width, height);
  context.strokeRect(-width / 2, -height / 2, width, height);

  if (node.status !== "ready") {
    context.strokeStyle = "rgba(255, 255, 255, 0.16)";
    context.lineWidth = 2;
    const spacing = Math.max(28, Math.min(80, width / 8));
    for (let offset = -height; offset < width; offset += spacing) {
      context.beginPath();
      context.moveTo(-width / 2 + offset, height / 2);
      context.lineTo(-width / 2 + offset + height, -height / 2);
      context.stroke();
    }
  }

  const labelInset = Math.max(14, Math.min(28, width * 0.04));
  const labelSize = Math.max(22, Math.min(46, height * 0.12));
  context.globalAlpha = 1;
  context.fillStyle = "#f4f8ff";
  context.font = `700 ${labelSize}px Inter, system-ui, sans-serif`;
  context.textBaseline = "top";
  context.fillText(
    node.name,
    -width / 2 + labelInset,
    -height / 2 + labelInset,
    Math.max(48, width - labelInset * 2),
  );
  context.font = `500 ${Math.max(16, labelSize * 0.58)}px Inter, system-ui, sans-serif`;
  context.fillStyle = "rgba(244, 248, 255, 0.72)";
  context.fillText(
    compositorNodeLabel(node, graph),
    -width / 2 + labelInset,
    -height / 2 + labelInset + labelSize + 8,
    Math.max(48, width - labelInset * 2),
  );
  context.restore();
}

function compositorNodeFill(node: CompositorNode): string {
  if (node.role === "video") return "rgba(36, 89, 204, 0.62)";
  if (node.role === "audio") return "rgba(33, 180, 145, 0.58)";
  if (node.role === "text") return "rgba(188, 78, 230, 0.55)";
  if (node.role === "group") return "rgba(130, 145, 175, 0.42)";
  return "rgba(207, 132, 42, 0.52)";
}

function compositorNodeStroke(node: CompositorNode): string {
  if (node.status === "permission_required") return "rgba(255, 210, 122, 0.86)";
  if (node.status === "unavailable") return "rgba(255, 105, 128, 0.82)";
  if (node.status === "placeholder") return "rgba(170, 188, 214, 0.76)";
  return "rgba(255, 255, 255, 0.28)";
}

function compositorNodeLabel(node: CompositorNode, graph: CompositorGraph): string {
  if (node.status !== "ready") return node.status_detail;
  return `${node.source_kind} - ${Math.round(node.transform.size.width)}x${Math.round(
    node.transform.size.height,
  )} on ${graph.output.width}x${graph.output.height}`;
}

function sceneSourcePreviewStyle(
  source: SceneSource,
  scene: Scene,
): CSSProperties {
  const left = (source.position.x / scene.canvas.width) * 100;
  const top = (source.position.y / scene.canvas.height) * 100;
  const width = (source.size.width / scene.canvas.width) * 100;
  const height = (source.size.height / scene.canvas.height) * 100;

  return {
    left: `${left}%`,
    top: `${top}%`,
    width: `${width}%`,
    height: `${height}%`,
    opacity: source.visible ? source.opacity : 0.26,
    transform: `rotate(${source.rotation_degrees}deg)`,
    zIndex: source.z_index,
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function sourceConfigSummary(source: SceneSource): string {
  switch (source.kind) {
    case "display":
      return source.config.resolution
        ? `${source.config.resolution.width}x${source.config.resolution.height}`
        : "Display pending";
    case "window":
      return source.config.title ?? source.config.application_name ?? "Window pending";
    case "camera":
      return source.config.resolution
        ? `${source.config.resolution.width}x${source.config.resolution.height} @ ${source.config.framerate ?? "auto"} fps`
        : "Camera pending";
    case "audio_meter":
      return `${source.config.channel} ${source.config.meter_style}`;
    case "image_media":
      return source.config.asset_uri ?? `No ${source.config.media_type} selected`;
    case "browser_overlay":
      return source.config.url ?? `${source.config.viewport.width}x${source.config.viewport.height} overlay`;
    case "text":
      return source.config.text;
    case "group":
      return `${source.config.child_source_ids.length} children`;
  }
}

function sceneSourceAvailability(source: SceneSource) {
  return "availability" in source.config ? source.config.availability : null;
}

function SourceKindIcon(props: { kind: SceneSourceKind }) {
  switch (props.kind) {
    case "display":
    case "window":
      return <Monitor size={15} />;
    case "camera":
      return <Video size={15} />;
    case "audio_meter":
      return <Mic size={15} />;
    case "image_media":
      return <ImageIcon size={15} />;
    case "browser_overlay":
      return <Globe size={15} />;
    case "text":
      return <Type size={15} />;
    case "group":
      return <Group size={15} />;
  }
}

function recordStudioMediaEvent(event: StudioEvent) {
  void recordSuiteTimelineEvent({
    kind: `studio.${event.type}`,
    title: studioMediaEventTitle(event),
    detail: studioMediaEventDetail(event),
    metadata: {
      ...event.payload,
      studioEventId: event.id,
    },
  });
}

function studioMediaEventTitle(event: StudioEvent): string {
  switch (event.type) {
    case "recording.started":
      return "Studio recording started";
    case "recording.stopped":
      return "Studio recording ready";
    case "stream.started":
      return "Studio stream started";
    case "stream.stopped":
      return "Studio stream stopped";
    default:
      return event.type;
  }
}

function studioMediaEventDetail(event: StudioEvent): string {
  const outputPath = String(event.payload.output_path ?? "");
  const destination = String(event.payload.destination_name ?? "");
  const sessionId = String(event.payload.session_id ?? "");
  if (outputPath) return outputPath;
  if (destination) return destination;
  if (sessionId) return sessionId;
  return "Studio media state changed.";
}

function ConnectedAppsPage(props: {
  clients: ConnectedClient[];
  config: RuntimeApiConfig | null;
  engine: string;
  mediaRunnerInfo: MediaRunnerInfo | null;
  onLaunchSuite: () => void;
  onStartSuiteSession: () => void;
  onSendSuiteCommand: (targetApp: string, command: string) => void;
  onReviewRecordingInPulse: (recording: RecordingHistoryEntry) => void;
  recentMarkers: Marker[];
  recentRecordings: RecordingHistoryEntry[];
  suiteSession: SuiteSession | null;
  suiteStatus: SuiteAppStatus[];
  suiteTimeline: SuiteTimelineItem[];
  suiteLaunchStatus: string | null;
}) {
  const apiUrl = props.config?.apiUrl ?? "http://127.0.0.1:51287";
  const wsUrl = props.config?.wsUrl ?? "ws://127.0.0.1:51287/events";
  const configuredApiUrl = props.config?.configuredApiUrl ?? apiUrl;
  const token = props.config?.token ?? "dev-auth-bypass";
  const runnerState = mediaRunnerState(props.mediaRunnerInfo, props.engine);
  const consolePlatformUrl = props.suiteStatus
    .find((app) => app.appId === "vaexcore-console")
    ?.apiUrl?.replace(/\/$/, "");

  return (
    <div className="stack">
      <section className="panel">
        <PanelTitle title="Suite Session" />
        <div className="suite-session-card">
          <div>
            <strong>{props.suiteSession?.title ?? "No active suite session"}</strong>
            <span>
              {props.suiteSession
                ? `Session ${props.suiteSession.sessionId}`
                : "Studio can create the shared local session used by all three apps."}
            </span>
          </div>
          <button
            className="secondary-button"
            onClick={props.onStartSuiteSession}
            type="button"
          >
            <RefreshCw size={16} />
            Start Session
          </button>
        </div>
      </section>
      <section className="panel">
        <PanelTitle title="Suite Presence" />
        {props.suiteStatus.length === 0 ? (
          <div className="empty">Suite status unavailable</div>
        ) : (
          <div className="table">
            {props.suiteStatus.map((app) => (
              <div className="table-row suite-status-row vxc-suite-row" key={app.appId}>
                <div>
                  <strong>{app.appName}</strong>
                  <span>{app.activityDetail ?? app.detail}</span>
                  <code>{app.healthUrl ?? app.discoveryFile}</code>
                </div>
                <Pill tone={app.suiteSessionId ? "green" : "muted"}>
                  {app.suiteSessionId ? "In session" : "No session"}
                </Pill>
                <Pill tone={app.activity ? "amber" : "muted"}>
                  {app.activity ?? "idle"}
                </Pill>
                <Pill tone={suiteStatusTone(app)}>
                  {suiteStatusLabel(app)}
                </Pill>
                <Pill tone={app.installed ? "green" : "red"}>
                  {app.installed ? "Installed" : "Missing"}
                </Pill>
                <Pill tone={app.running ? "green" : "muted"}>
                  {app.running ? "Running" : "Stopped"}
                </Pill>
              </div>
            ))}
          </div>
        )}
      </section>
      <section className="panel">
        <PanelTitle title="Suite Launcher" />
        <button
          className="secondary-button full"
          onClick={props.onLaunchSuite}
          type="button"
        >
          <Play size={16} />
          Launch & Verify Studio, Pulse, and Console
        </button>
        {props.suiteLaunchStatus && (
          <Pill tone={suiteLaunchTone(props.suiteLaunchStatus)}>
            {props.suiteLaunchStatus}
          </Pill>
        )}
        <div className="button-row">
          <button
            className="secondary-button compact"
            onClick={() => props.onSendSuiteCommand("vaexcore-pulse", "focus-review")}
            type="button"
          >
            <Link2 size={14} />
            Pulse Review
          </button>
          <button
            className="secondary-button compact"
            onClick={() => props.onSendSuiteCommand("vaexcore-pulse", "focus-suite")}
            type="button"
          >
            <Cable size={14} />
            Pulse Suite
          </button>
          <button
            className="secondary-button compact"
            onClick={() => props.onSendSuiteCommand("vaexcore-console", "focus-ops")}
            type="button"
          >
            <Terminal size={14} />
            Console Ops
          </button>
        </div>
      </section>
      <section className="panel">
        <PanelTitle title="Suite Timeline" />
        {props.suiteTimeline.length === 0 ? (
          <div className="empty">No shared activity yet</div>
        ) : (
          <div className="table">
            {props.suiteTimeline.map((item) => (
              <div className="table-row vxc-timeline-row" key={item.id}>
                <div>
                  <strong>{item.title}</strong>
                  <span>{item.detail}</span>
                </div>
                <Pill tone={timelineTone(item.kind)}>{item.source}</Pill>
                <Pill tone="muted">{formatSuiteTimestamp(item.timestamp)}</Pill>
              </div>
            ))}
          </div>
        )}
      </section>
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
        {consolePlatformUrl && (
          <button
            className="secondary-button full"
            onClick={() => window.open(`${consolePlatformUrl}/platform`, "_blank")}
            type="button"
          >
            <Link2 size={16} />
            Open Platform Page
          </button>
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
                  <strong>{connectedClientName(client)}</strong>
                  <span>{client.last_path ?? "local API"}</span>
                </div>
                <Pill tone={connectedClientTone(client)}>
                  {connectedClientApp(client)}
                </Pill>
                <Pill tone={client.kind === "websocket" ? "green" : "amber"}>
                  {client.kind}
                </Pill>
                <span>
                  {client.request_count} req,{" "}
                  {new Date(client.last_seen_at).toLocaleTimeString()}
                </span>
              </div>
            ))}
          </div>
        )}
      </section>
      <section className="panel">
        <PanelTitle title="Recent Recordings" />
        {props.recentRecordings.length === 0 ? (
          <div className="empty">No completed recordings yet</div>
        ) : (
          <div className="table">
            {props.recentRecordings.map((recording) => (
              <div className="table-row" key={recording.session_id}>
                <div>
                  <strong>{recording.profile_name}</strong>
                  <span>{recording.session_id}</span>
                  <code>{recording.output_path}</code>
                </div>
                <Pill tone="amber">
                  {new Date(recording.stopped_at).toLocaleTimeString()}
                </Pill>
                <Pill tone="muted">{recording.profile_id}</Pill>
                <div className="table-actions">
                  <button
                    className="secondary-button compact"
                    onClick={() => props.onReviewRecordingInPulse(recording)}
                    type="button"
                  >
                    <Play size={14} />
                    Review in Pulse
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </section>
      <section className="panel">
        <PanelTitle title="Recent Markers" />
        {props.recentMarkers.length === 0 ? (
          <div className="empty">No markers yet</div>
        ) : (
          <div className="table">
            {props.recentMarkers.map((marker) => (
              <div className="table-row" key={marker.id}>
                <div>
                  <strong>{marker.label ?? "Untitled marker"}</strong>
                  <span>{marker.source_event_id ?? marker.id}</span>
                  {marker.media_path && <code>{marker.media_path}</code>}
                </div>
                <Pill tone={markerSourceTone(marker.source_app)}>
                  {markerSourceLabel(marker.source_app)}
                </Pill>
                <Pill tone="muted">
                  {marker.start_seconds !== null && marker.end_seconds !== null
                    ? `${marker.start_seconds.toFixed(1)}-${marker.end_seconds.toFixed(1)}s`
                    : new Date(marker.created_at).toLocaleTimeString()}
                </Pill>
              </div>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}

function formatSuiteLaunchFailure(results: SuiteLaunchResult[]): string {
  const appNames = results.map((result) => result.appName).join(", ");
  return `Could not launch ${appNames}. Install the app bundles in Applications, then try again.`;
}

function buildSuiteTimeline(
  suiteStatus: SuiteAppStatus[],
  recordings: RecordingHistoryEntry[],
  markers: Marker[],
  events: StudioEvent[],
  persistedEvents: SuiteTimelineEvent[],
): SuiteTimelineItem[] {
  const persistedItems = persistedEvents.map((event) => ({
    id: `persisted-${event.eventId}`,
    kind: suiteTimelineItemKind(event.kind),
    title: event.title,
    detail: event.detail,
    timestamp: event.createdAt,
    source: event.sourceAppName,
  }));
  const presence = suiteStatus
    .filter((app) => app.updatedAt)
    .map((app) => ({
      id: `presence-${app.appId}-${app.updatedAt}`,
      kind: "presence" as const,
      title: app.activity ?? app.appName,
      detail: app.activityDetail ?? app.detail,
      timestamp: app.updatedAt ?? new Date().toISOString(),
      source: app.appName,
    }));
  const recordingItems = recordings.map((recording) => ({
    id: `recording-${recording.session_id}`,
    kind: "recording" as const,
    title: `Recording ready: ${recording.profile_name}`,
    detail: recording.output_path,
    timestamp: recording.stopped_at,
    source: "Studio",
  }));
  const markerItems = markers.map((marker) => ({
    id: `marker-${marker.id}`,
    kind: "marker" as const,
    title: marker.label ?? "Marker",
    detail: marker.media_path ?? marker.source_event_id ?? marker.id,
    timestamp: marker.created_at,
    source: markerSourceLabel(marker.source_app),
  }));
  const eventItems = events.slice(0, 10).map((event) => ({
    id: `event-${event.id}`,
    kind: "event" as const,
    title: event.type,
    detail: String(
      event.payload["session_id"] ?? event.payload["destination_name"] ?? event.id,
    ),
    timestamp: event.timestamp,
    source: "Studio",
  }));

  return [...persistedItems, ...presence, ...recordingItems, ...markerItems, ...eventItems]
    .sort((left, right) => suiteTimestampMs(right.timestamp) - suiteTimestampMs(left.timestamp))
    .slice(0, 18);
}

function suiteTimelineItemKind(kind: string): SuiteTimelineItem["kind"] {
  if (kind.includes("recording")) return "recording";
  if (kind.includes("marker")) return "marker";
  if (kind.includes("presence") || kind.includes("session")) return "presence";
  return "event";
}

function suiteTimestampMs(value: string): number {
  if (/^\d+$/.test(value)) {
    return Number(value) * 1000;
  }
  const parsed = Date.parse(value);
  return Number.isNaN(parsed) ? 0 : parsed;
}

function formatSuiteTimestamp(value: string): string {
  const timestamp = suiteTimestampMs(value);
  if (!timestamp) return value;
  return new Date(timestamp).toLocaleTimeString();
}

function timelineTone(kind: SuiteTimelineItem["kind"]): "green" | "red" | "amber" | "muted" {
  if (kind === "presence") return "green";
  if (kind === "recording") return "amber";
  if (kind === "marker") return "red";
  return "muted";
}

function formatSuiteVerification(status: SuiteAppStatus[]): string {
  const blocked = status.filter(
    (app) => !app.installed || !app.running || app.stale || !app.reachable,
  );
  if (blocked.length === 0 && status.length > 0) {
    return "Suite verified. Studio, Pulse, and Console are ready.";
  }
  if (blocked.length === 0) {
    return "Launch requested for Studio, Pulse, and Console.";
  }
  return `Launch requested. Still waiting on ${blocked
    .map((app) => app.appName)
    .join(", ")}.`;
}

function suiteLaunchTone(status: string): "green" | "red" | "amber" | "muted" {
  if (status.startsWith("Could not")) return "red";
  if (status.includes("waiting") || status.includes("Verifying")) return "amber";
  return "green";
}

function suiteStatusTone(app: SuiteAppStatus): "green" | "red" | "amber" | "muted" {
  if (!app.installed) return "red";
  if (!app.running) return "muted";
  if (app.stale || !app.reachable) return "amber";
  return "green";
}

function suiteStatusLabel(app: SuiteAppStatus): string {
  if (!app.installed) return "Missing";
  if (!app.running) return "Offline";
  if (app.stale) return "Stale";
  if (!app.reachable) return "Starting";
  return "Ready";
}

function markerSourceLabel(sourceApp: string | null): string {
  if (sourceApp === "vaexcore-pulse") return "Pulse";
  if (sourceApp === "vaexcore-console") return "Console";
  if (sourceApp) return sourceApp;
  return "Studio";
}

function markerSourceTone(
  sourceApp: string | null,
): "green" | "red" | "amber" | "muted" {
  if (sourceApp === "vaexcore-pulse" || sourceApp === "vaexcore-console") {
    return "green";
  }
  if (sourceApp) return "amber";
  return "muted";
}

function connectedClientName(client: ConnectedClient): string {
  if (client.id.includes("vaexcore-pulse")) return "vaexcore pulse";
  if (client.id.includes("vaexcore-console")) return "VaexCore Console";
  return client.name;
}

function connectedClientApp(client: ConnectedClient): string {
  const label = `${client.id} ${client.name}`.toLowerCase();
  if (label.includes("pulse")) return "Pulse";
  if (label.includes("console")) return "Console";
  if (label.includes("studio")) return "Studio";
  return "External";
}

function connectedClientTone(
  client: ConnectedClient,
): "green" | "red" | "amber" | "muted" {
  const app = connectedClientApp(client);
  if (app === "Pulse" || app === "Console") return "green";
  if (app === "Studio") return "muted";
  return "amber";
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
    props.onSettingsChange(
      updateSettingsCaptureSource(props.settings, candidate, enabled),
    );
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

function SceneNumberInput(props: {
  label: string;
  max?: number;
  min?: number;
  onChange: (value: number) => void;
  step?: number;
  value: number;
}) {
  return (
    <label>
      {props.label}
      <input
        max={props.max}
        min={props.min}
        step={props.step ?? 1}
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

function initialSection(): Section {
  const requested = new URLSearchParams(window.location.search).get("section");
  return isSection(requested) ? requested : "dashboard";
}

function sectionTitle(section: Section): string {
  switch (section) {
    case "dashboard":
      return "Control Room";
    case "designer":
      return "Designer";
    case "destinations":
      return "Broadcast Destinations";
    case "profiles":
      return "Recording Profiles";
    case "controls":
      return "Broadcast Setup";
    case "apps":
      return "Suite";
    case "logs":
      return "Event Log";
  }
}

function sectionHeading(section: Section): string {
  switch (section) {
    case "dashboard":
      return "Studio Control Room";
    case "designer":
      return "Scene Designer";
    case "destinations":
      return "Broadcast Destinations";
    case "profiles":
      return "Recording Profiles";
    case "controls":
      return "Broadcast Setup";
    case "apps":
      return "Suite Presence";
    case "logs":
      return "Event Log";
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

function updateSettingsCaptureSource(
  settings: AppSettings,
  candidate: CaptureSourceCandidate,
  enabled: boolean,
): AppSettings {
  const existing = settings.capture_sources.find(
    (source) => source.id === candidate.id,
  );
  const nextSource: CaptureSourceSelection = {
    id: candidate.id,
    kind: candidate.kind,
    name: candidate.name,
    enabled,
  };
  const capture_sources = existing
    ? settings.capture_sources.map((source) =>
        source.id === candidate.id ? { ...source, enabled } : source,
      )
    : [...settings.capture_sources, nextSource];

  return { ...settings, capture_sources };
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
