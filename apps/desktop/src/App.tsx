import {
  Activity,
  Cable,
  CheckCircle2,
  Copy,
  FileVideo,
  MapPin,
  Play,
  Plus,
  Radio,
  ScrollText,
  SlidersHorizontal,
  Square,
  Terminal,
  Video,
  WifiOff,
} from "lucide-react";
import { FormEvent, ReactNode, useEffect, useMemo, useState } from "react";
import type {
  HealthResponse,
  MediaProfileInput,
  PlatformKind,
  ProfilesSnapshot,
  RecordingContainer,
  StudioEvent,
  StudioStatus,
  StreamDestinationInput,
} from "@vaexcore/shared-types";
import { platformLabels } from "@vaexcore/shared-types";
import {
  eventSocketUrl,
  loadRuntimeConfig,
  RuntimeApiConfig,
  StudioApi,
} from "./api";
import logoUrl from "./assets/brand/vaexcore-studio-logo.jpg";

type Section =
  | "dashboard"
  | "destinations"
  | "profiles"
  | "controls"
  | "apps"
  | "logs"
  | "settings";

const navItems: Array<{ id: Section; label: string; icon: ReactNode }> = [
  { id: "dashboard", label: "Dashboard", icon: <Activity size={17} /> },
  { id: "destinations", label: "Stream Destinations", icon: <Radio size={17} /> },
  { id: "profiles", label: "Recording Profiles", icon: <FileVideo size={17} /> },
  { id: "controls", label: "Controls", icon: <SlidersHorizontal size={17} /> },
  { id: "apps", label: "Connected Apps", icon: <Cable size={17} /> },
  { id: "logs", label: "Logs", icon: <ScrollText size={17} /> },
];

const openSettingsEvent = "vaexcore://open-settings";

const defaultProfileForm: MediaProfileInput = {
  name: "1080p60 Local",
  output_folder: "~/Movies/vaexcore-studio",
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

function App() {
  const [section, setSection] = useState<Section>("dashboard");
  const [config, setConfig] = useState<RuntimeApiConfig | null>(null);
  const [health, setHealth] = useState<HealthResponse | null>(null);
  const [status, setStatus] = useState<StudioStatus | null>(null);
  const [profiles, setProfiles] = useState<ProfilesSnapshot | null>(null);
  const [events, setEvents] = useState<StudioEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [selectedProfileId, setSelectedProfileId] = useState<string | undefined>();
  const [selectedDestinationId, setSelectedDestinationId] = useState<string | undefined>();
  const [profileForm, setProfileForm] = useState<MediaProfileInput>(defaultProfileForm);
  const [destinationForm, setDestinationForm] =
    useState<StreamDestinationInput>(defaultDestinationForm);
  const [markerLabel, setMarkerLabel] = useState("manual-marker");

  useEffect(() => {
    loadRuntimeConfig().then(setConfig).catch((error: Error) => {
      setError(error.message);
    });
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;

    import("@tauri-apps/api/event")
      .then(({ listen }) =>
        listen(openSettingsEvent, () => {
          setSection("settings");
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
        const [nextHealth, nextStatus, nextProfiles] = await Promise.all([
          StudioApi.health(runtimeConfig),
          StudioApi.status(runtimeConfig),
          StudioApi.profiles(runtimeConfig),
        ]);
        if (cancelled) return;
        setHealth(nextHealth);
        setStatus(nextStatus);
        setProfiles(nextProfiles);
        setEvents((current) =>
          mergeEvents([...nextStatus.recent_events, ...current]),
        );
        setError(null);
        setSelectedProfileId(
          (current) => current ?? nextProfiles.recording_profiles[0]?.id,
        );
        setSelectedDestinationId(
          (current) => current ?? nextProfiles.stream_destinations[0]?.id,
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
    setSelectedProfileId(
      (current) => current ?? nextProfiles.recording_profiles[0]?.id,
    );
    setSelectedDestinationId(
      (current) => current ?? nextProfiles.stream_destinations[0]?.id,
    );
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
      await StudioApi.createProfile(config, {
        kind: "recording_profile",
        value: profileForm,
      });
      await refreshProfiles();
      setError(null);
    } catch (error) {
      setError(error instanceof Error ? error.message : "Profile create failed");
    }
  }

  async function createDestination(event: FormEvent) {
    event.preventDefault();
    if (!config) return;
    try {
      await StudioApi.createProfile(config, {
        kind: "stream_destination",
        value: {
          ...destinationForm,
          stream_key: destinationForm.stream_key || null,
        },
      });
      await refreshProfiles();
      setError(null);
    } catch (error) {
      setError(
        error instanceof Error ? error.message : "Destination create failed",
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
            recordingActive={activeStatus?.recording_active ?? false}
            recordingPath={recordingPath ?? "No active recording"}
            streamActive={activeStatus?.stream_active ?? false}
          />
        );
      case "destinations":
        return (
          <DestinationsPage
            destinationForm={destinationForm}
            onCreate={createDestination}
            onFormChange={setDestinationForm}
            profiles={profiles}
          />
        );
      case "profiles":
        return (
          <RecordingProfilesPage
            onCreate={createRecordingProfile}
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
        return <ConnectedAppsPage config={config} />;
      case "logs":
        return <LogsPage events={events} />;
      case "settings":
        return (
          <SettingsPage
            config={config}
            engine={activeStatus?.engine ?? "starting"}
            health={health}
            logoUrl={logoUrl}
            mode={activeStatus?.mode ?? "dry_run"}
          />
        );
    }
  }, [
    activeDestination?.name,
    activeStatus?.engine,
    activeStatus?.mode,
    activeStatus?.recording_active,
    activeStatus?.stream_active,
    config,
    destinationForm,
    events,
    health,
    markerLabel,
    profileForm,
    profiles,
    recordingPath,
    section,
    selectedDestinationId,
    selectedProfileId,
  ]);

  return (
    <main className="shell">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-mark">
            <img alt="" src={logoUrl} />
          </div>
          <div>
            <h1>vaexcore-studio</h1>
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
          <StatusDot active={!error} />
          <span>{error ? "API unavailable" : "API connected"}</span>
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
    </div>
  );
}

function DestinationsPage(props: {
  destinationForm: StreamDestinationInput;
  onCreate: (event: FormEvent) => void;
  onFormChange: (value: StreamDestinationInput) => void;
  profiles: ProfilesSnapshot | null;
}) {
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
            </div>
          ))}
        </div>
      </section>

      <section className="panel">
        <PanelTitle title="Create Destination" />
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
          <button className="primary-button" type="submit">
            <Plus size={16} />
            Add Destination
          </button>
        </form>
      </section>
    </div>
  );
}

function RecordingProfilesPage(props: {
  onCreate: (event: FormEvent) => void;
  onFormChange: (value: MediaProfileInput) => void;
  profileForm: MediaProfileInput;
  profiles: ProfilesSnapshot | null;
}) {
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
            </div>
          ))}
        </div>
      </section>

      <section className="panel">
        <PanelTitle title="Create Profile" />
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
          <button className="primary-button" type="submit">
            <Plus size={16} />
            Add Profile
          </button>
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

function ConnectedAppsPage(props: { config: RuntimeApiConfig | null }) {
  const apiUrl = props.config?.apiUrl ?? "http://127.0.0.1:51287";
  const wsUrl = props.config?.wsUrl ?? "ws://127.0.0.1:51287/events";
  const token = props.config?.token ?? "dev-auth-bypass";

  return (
    <div className="stack">
      <section className="panel">
        <PanelTitle title="Local Endpoints" />
        <CopyLine label="HTTP API URL" value={apiUrl} />
        <CopyLine label="WebSocket URL" value={wsUrl} />
        <CopyLine label="API Token" secret value={token} />
      </section>
      <section className="panel">
        <PanelTitle title="Recent Clients" />
        <div className="table">
          {["Twitch bot", "Highlight locator", "Stream deck bridge"].map(
            (client) => (
              <div className="table-row" key={client}>
                <div>
                  <strong>{client}</strong>
                  <span>placeholder client registry</span>
                </div>
                <Pill tone="muted">not connected</Pill>
              </div>
            ),
          )}
        </div>
      </section>
    </div>
  );
}

function LogsPage(props: { events: StudioEvent[] }) {
  return (
    <section className="panel">
      <PanelTitle title="Event Log" />
      <EventList events={props.events} />
    </section>
  );
}

function SettingsPage(props: {
  config: RuntimeApiConfig | null;
  engine: string;
  health: HealthResponse | null;
  logoUrl: string;
  mode: string;
}) {
  return (
    <div className="settings-grid">
      <section className="panel identity-panel">
        <img alt="" src={props.logoUrl} />
        <div>
          <PanelTitle title="Identity" />
          <KeyValue label="Product" value="vaexcore-studio" />
          <KeyValue label="Role" value="local foundation layer" />
        </div>
      </section>
      <section className="panel">
        <PanelTitle title="Runtime" />
        <KeyValue label="Engine" value={props.engine} />
        <KeyValue label="Mode" value={props.mode} />
        <KeyValue
          label="Service"
          value={props.health?.service ?? "vaexcore-studio"}
        />
        <KeyValue label="Version" value={props.health?.version ?? "0.1.0"} />
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
    </div>
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

function PanelTitle(props: { title: string }) {
  return (
    <div className="panel-title">
      <h3>{props.title}</h3>
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
    case "settings":
      return "Settings";
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

function mask(value: string): string {
  if (value.length <= 8) return "********";
  return `${value.slice(0, 4)}****${value.slice(-4)}`;
}

export default App;
