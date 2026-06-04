import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  AppStream,
  BusDef,
  MicConfig,
  OutputDevice,
  ProfileInfo,
  SeenApp,
  VirtualSink,
} from "../types";

// Faders fire on every pointer move; debounce backend calls per target so a
// drag doesn't spawn a pactl subprocess per pixel. UI state updates
// optimistically and immediately.
const pendingInvokes = new Map<string, number>();
function debouncedInvoke(key: string, cmd: string, args: Record<string, unknown>, onError: (e: unknown) => void) {
  const existing = pendingInvokes.get(key);
  if (existing !== undefined) clearTimeout(existing);
  pendingInvokes.set(
    key,
    window.setTimeout(() => {
      pendingInvokes.delete(key);
      invoke(cmd, args).catch(onError);
    }, 90),
  );
}

/** Per-sink [left, right] peak amplitudes (0–1), streamed from the native backend. */
export type Levels = Record<string, [number, number]>;

interface MixerStore {
  channels: VirtualSink[];
  appStreams: AppStream[];
  /** Live VU levels; stays empty under the pactl fallback backend. */
  levels: Levels;
  setLevels: (levels: Levels) => void;
  /** Physical output devices. */
  outputDevices: OutputDevice[];
  /** Channel -> chosen output node name (null = follow system default). */
  channelOutputs: Record<string, string | null>;
  fetchOutputs: () => Promise<void>;
  setChannelOutput: (sinkName: string, outputName: string | null) => Promise<void>;
  /** Sonar-style "same device on all channels". */
  setAllOutputs: (outputName: string | null) => Promise<void>;
  /** Mic chain (Phase 3). Null until loaded. */
  micConfig: MicConfig | null;
  inputDevices: OutputDevice[];
  fetchMic: () => Promise<void>;
  setMicConfig: (patch: Partial<MicConfig>) => Promise<void>;
  profiles: ProfileInfo[];
  /** Bind/clear an output device that auto-loads a profile (Phase 5). */
  setProfileTrigger: (name: string, device: string | null) => Promise<void>;
  /** Create a clean-slate profile (saved, not applied). */
  createBlankProfile: (name: string) => Promise<void>;
  /** A profile was switched outside the UI (tray) — sync everything. */
  onProfileChanged: (name: string) => Promise<void>;
  /** App history (live + gone + ignored). */
  seenApps: SeenApp[];
  fetchSeenApps: () => Promise<void>;
  setAppIgnored: (app: { match_prop: string; match_value: string }, ignored: boolean) => Promise<void>;
  forgetApp: (app: { match_prop: string; match_value: string }) => Promise<void>;
  /** Pre-route an app that isn't currently running (null clears). */
  setAppAssignment: (
    app: { match_prop: string; match_value: string },
    sinkName: string | null,
  ) => Promise<void>;
  /** Channel management: labels are free-form, sink names are stable. */
  addChannel: (label: string, icon: string | null) => Promise<void>;
  renameChannel: (sinkName: string, label: string) => Promise<void>;
  removeChannel: (sinkName: string) => Promise<void>;
  setChannelIcon: (sinkName: string, icon: string) => Promise<void>;
  /** User-defined mixes (record buses). */
  buses: BusDef[];
  fetchBuses: () => Promise<void>;
  addBus: (label: string) => Promise<void>;
  renameBus: (name: string, label: string) => Promise<void>;
  removeBus: (name: string) => Promise<void>;
  setBusMembers: (name: string, channels: string[]) => Promise<void>;
  /** Session-scoped "listen on default output" toggles per node. */
  monitors: Record<string, boolean>;
  toggleMonitor: (name: string) => Promise<void>;
  /** Name of the most recently saved/loaded profile this session. */
  activeProfile: string | null;
  /** Fatal error surfaced to the UI (e.g. pactl missing, PipeWire down). */
  error: string | null;
  initialized: boolean;
  /** True on the native PipeWire backend; false on the pactl fallback
   * (mixes/mic/monitoring unavailable). Null until known. */
  backendNative: boolean | null;

  /** Create the virtual sinks and load initial state. */
  initialize: () => Promise<void>;
  fetchChannels: () => Promise<void>;
  fetchAppStreams: () => Promise<void>;
  setChannelVolume: (sinkName: string, volume: number) => Promise<void>;
  toggleMute: (sinkName: string, muted: boolean) => Promise<void>;
  routeApp: (streamIndex: number, sinkName: string) => Promise<void>;
  setAppVolume: (streamIndex: number, volume: number) => Promise<void>;
  fetchProfiles: () => Promise<void>;
  loadProfile: (name: string) => Promise<void>;
  deleteProfile: (name: string) => Promise<void>;
  /** Set or clear (empty string) a persistent display name for an app. */
  renameApp: (stream: AppStream, alias: string) => Promise<void>;
}

export const useMixerStore = create<MixerStore>((set, get) => ({
  channels: [],
  appStreams: [],
  levels: {},
  setLevels: (levels) => {
    // Levels arrive at 10 Hz even when everything is silent; skipping
    // no-op updates avoids re-rendering every strip 10×/second at idle.
    const prev = get().levels;
    const keys = Object.keys(levels);
    const unchanged =
      keys.length === Object.keys(prev).length &&
      keys.every((k) => {
        const a = prev[k];
        const b = levels[k];
        return a && Math.abs(a[0] - b[0]) < 1e-4 && Math.abs(a[1] - b[1]) < 1e-4;
      });
    if (!unchanged) set({ levels });
  },
  outputDevices: [],
  channelOutputs: {},
  micConfig: null,
  inputDevices: [],
  seenApps: [],
  profiles: [],
  activeProfile: null,
  error: null,
  initialized: false,
  backendNative: null,

  initialize: async () => {
    if (get().initialized) return;
    try {
      await invoke("init_virtual_devices");
      set({ initialized: true, error: null });
      void invoke<{ native: boolean }>("get_backend_info")
        .then((i) => set({ backendNative: i.native }))
        .catch(() => {});
      await Promise.all([
        get().fetchChannels(),
        get().fetchAppStreams(),
        get().fetchProfiles(),
        get().fetchOutputs(),
        get().fetchMic(),
        get().fetchBuses(),
      ]);
      // Active profile is tracked backend-side (survives restarts).
      try {
        const active = await invoke<string | null>("get_active_profile");
        if (active) {
          set({ activeProfile: active });
        } else if (get().profiles.some((p) => p.name === "Default")) {
          // First run: the backend just created "Default" from this layout.
          set({ activeProfile: "Default" });
        }
      } catch {
        /* older backend without the command — banner-worthy errors surface elsewhere */
      }
    } catch (e) {
      set({ error: String(e) });
    }
  },

  fetchChannels: async () => {
    try {
      const channels = await invoke<VirtualSink[]>("get_virtual_devices");
      set({ channels });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  fetchAppStreams: async () => {
    try {
      const appStreams = await invoke<AppStream[]>("get_app_streams");
      set({ appStreams, error: null });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setChannelVolume: async (sinkName, volume) => {
    set((s) => ({
      channels: s.channels.map((c) =>
        c.name === sinkName ? { ...c, volume_percent: volume } : c,
      ),
    }));
    debouncedInvoke(
      `chvol:${sinkName}`,
      "set_channel_volume",
      { sinkName, volume },
      (e) => {
        set({ error: String(e) });
        void get().fetchChannels();
      },
    );
  },

  toggleMute: async (sinkName, muted) => {
    set((s) => ({
      channels: s.channels.map((c) =>
        c.name === sinkName ? { ...c, muted } : c,
      ),
    }));
    try {
      await invoke("toggle_channel_mute", { sinkName, muted });
    } catch (e) {
      set({ error: String(e) });
      await get().fetchChannels();
    }
  },

  routeApp: async (streamIndex, sinkName) => {
    set((s) => ({
      appStreams: s.appStreams.map((a) =>
        a.index === streamIndex
          ? { ...a, assigned_sink: sinkName === "" ? null : sinkName }
          : a,
      ),
    }));
    try {
      await invoke("route_app_to_channel", { streamIndex, sinkName });
    } catch (e) {
      set({ error: String(e) });
    } finally {
      await get().fetchAppStreams();
    }
  },

  setAppVolume: async (streamIndex, volume) => {
    set((s) => ({
      appStreams: s.appStreams.map((a) =>
        a.index === streamIndex ? { ...a, volume_percent: volume } : a,
      ),
    }));
    debouncedInvoke(
      `appvol:${streamIndex}`,
      "set_app_volume",
      { streamIndex, volume },
      (e) => set({ error: String(e) }),
    );
  },

  fetchOutputs: async () => {
    try {
      const [outputDevices, channelOutputs] = await Promise.all([
        invoke<OutputDevice[]>("get_output_devices"),
        invoke<Record<string, string | null>>("get_channel_outputs"),
      ]);
      set({ outputDevices, channelOutputs });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setChannelOutput: async (sinkName, outputName) => {
    set((s) => ({
      channelOutputs: { ...s.channelOutputs, [sinkName]: outputName },
    }));
    try {
      await invoke("set_channel_output", { sinkName, outputName: outputName ?? "" });
    } catch (e) {
      set({ error: String(e) });
      await get().fetchOutputs();
    }
  },

  setAllOutputs: async (outputName) => {
    for (const channel of get().channels) {
      await get().setChannelOutput(channel.name, outputName);
    }
  },

  fetchMic: async () => {
    try {
      const [micConfig, inputDevices] = await Promise.all([
        invoke<MicConfig>("get_mic_config"),
        invoke<OutputDevice[]>("get_input_devices"),
      ]);
      set({ micConfig, inputDevices });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setMicConfig: async (patch) => {
    const current = get().micConfig;
    if (!current) return;
    const config = { ...current, ...patch };
    set({ micConfig: config });
    // Debounced: slider drags and rename typing settle into one apply.
    debouncedInvoke("micConfig", "set_mic_config", { config }, (e) => {
      set({ error: String(e) });
      void get().fetchMic();
    });
  },

  fetchProfiles: async () => {
    try {
      const profiles = await invoke<ProfileInfo[]>("list_profiles");
      set({ profiles });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setProfileTrigger: async (name, device) => {
    try {
      await invoke("set_profile_trigger", { name, device: device ?? "" });
      await get().fetchProfiles();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  onProfileChanged: async (name) => {
    set({ activeProfile: name });
    await Promise.all([
      get().fetchChannels(),
      get().fetchAppStreams(),
      get().fetchOutputs(),
      get().fetchSeenApps(),
      get().fetchProfiles(),
      get().fetchBuses(),
    ]);
  },

  createBlankProfile: async (name) => {
    try {
      await invoke("create_blank_profile", { name });
      await get().fetchProfiles();
      // Switch to the fresh profile right away — creating a blank slate
      // and not seeing anything change reads as a bug.
      await get().loadProfile(name);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  fetchSeenApps: async () => {
    try {
      const seenApps = await invoke<SeenApp[]>("get_seen_apps");
      set({ seenApps });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setAppIgnored: async (app, ignored) => {
    try {
      await invoke("set_app_ignored", {
        matchProp: app.match_prop,
        matchValue: app.match_value,
        ignored,
      });
      await Promise.all([get().fetchSeenApps(), get().fetchAppStreams()]);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  forgetApp: async (app) => {
    try {
      await invoke("forget_app", {
        matchProp: app.match_prop,
        matchValue: app.match_value,
      });
      await get().fetchSeenApps();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setAppAssignment: async (app, sinkName) => {
    try {
      await invoke("set_app_assignment", {
        matchProp: app.match_prop,
        matchValue: app.match_value,
        sinkName: sinkName ?? "",
      });
      await get().fetchSeenApps();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  loadProfile: async (name) => {
    try {
      await invoke("load_profile", { name });
      set({ activeProfile: name });
      // Layout, volumes and routing all changed backend-side.
      await Promise.all([
        get().fetchChannels(),
        get().fetchAppStreams(),
        get().fetchOutputs(),
        get().fetchSeenApps(),
        get().fetchBuses(),
      ]);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  deleteProfile: async (name) => {
    try {
      await invoke("delete_profile", { name });
      if (get().activeProfile === name) set({ activeProfile: null });
      await get().fetchProfiles();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  addChannel: async (label, icon) => {
    try {
      await invoke("add_channel", { label, icon });
      await Promise.all([get().fetchChannels(), get().fetchOutputs()]);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  buses: [],

  fetchBuses: async () => {
    try {
      const buses = await invoke<BusDef[]>("list_buses");
      set({ buses });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  addBus: async (label) => {
    try {
      await invoke("add_bus", { label });
      await get().fetchBuses();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  renameBus: async (name, label) => {
    set((s) => ({
      buses: s.buses.map((b) => (b.name === name ? { ...b, label } : b)),
    }));
    try {
      await invoke("rename_bus", { name, label });
    } catch (e) {
      set({ error: String(e) });
      await get().fetchBuses();
    }
  },

  removeBus: async (name) => {
    try {
      await invoke("remove_bus", { name });
      await get().fetchBuses();
    } catch (e) {
      set({ error: String(e) });
    }
  },

  setBusMembers: async (name, channels) => {
    set((s) => ({
      buses: s.buses.map((b) => (b.name === name ? { ...b, channels } : b)),
    }));
    try {
      await invoke("set_bus_members", { name, channels });
    } catch (e) {
      set({ error: String(e) });
      await get().fetchBuses();
    }
  },

  monitors: {},

  toggleMonitor: async (name) => {
    const enabled = !get().monitors[name];
    set((s) => ({ monitors: { ...s.monitors, [name]: enabled } }));
    try {
      await invoke("set_monitor", { sinkName: name, enabled });
    } catch (e) {
      set({ error: String(e) });
      set((s) => ({ monitors: { ...s.monitors, [name]: !enabled } }));
    }
  },

  setChannelIcon: async (sinkName, icon) => {
    set((s) => ({
      channels: s.channels.map((c) => (c.name === sinkName ? { ...c, icon } : c)),
    }));
    try {
      await invoke("set_channel_icon", { sinkName, icon });
    } catch (e) {
      set({ error: String(e) });
      await get().fetchChannels();
    }
  },

  renameChannel: async (sinkName, label) => {
    set((s) => ({
      channels: s.channels.map((c) => (c.name === sinkName ? { ...c, label } : c)),
    }));
    try {
      await invoke("rename_channel", { sinkName, label });
    } catch (e) {
      set({ error: String(e) });
      await get().fetchChannels();
    }
  },

  removeChannel: async (sinkName) => {
    try {
      await invoke("remove_channel", { sinkName });
      await Promise.all([
        get().fetchChannels(),
        get().fetchAppStreams(),
        get().fetchOutputs(),
      ]);
    } catch (e) {
      set({ error: String(e) });
    }
  },

  renameApp: async (stream, alias) => {
    const trimmed = alias.trim();
    set((s) => ({
      appStreams: s.appStreams.map((a) =>
        a.match_prop === stream.match_prop && a.match_value === stream.match_value
          ? { ...a, alias: trimmed === "" ? null : trimmed }
          : a,
      ),
    }));
    try {
      await invoke("rename_app", {
        matchProp: stream.match_prop,
        matchValue: stream.match_value,
        alias: trimmed,
      });
    } catch (e) {
      set({ error: String(e) });
      await get().fetchAppStreams();
    }
  },
}));
