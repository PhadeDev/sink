import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { AppStream, VirtualSink } from "../types";

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

interface MixerStore {
  channels: VirtualSink[];
  appStreams: AppStream[];
  /** Fatal error surfaced to the UI (e.g. pactl missing, PipeWire down). */
  error: string | null;
  initialized: boolean;

  /** Create the virtual sinks and load initial state. */
  initialize: () => Promise<void>;
  fetchChannels: () => Promise<void>;
  fetchAppStreams: () => Promise<void>;
  setChannelVolume: (sinkName: string, volume: number) => Promise<void>;
  toggleMute: (sinkName: string, muted: boolean) => Promise<void>;
  routeApp: (streamIndex: number, sinkName: string) => Promise<void>;
  setAppVolume: (streamIndex: number, volume: number) => Promise<void>;
}

export const useMixerStore = create<MixerStore>((set, get) => ({
  channels: [],
  appStreams: [],
  error: null,
  initialized: false,

  initialize: async () => {
    if (get().initialized) return;
    try {
      await invoke("init_virtual_devices");
      set({ initialized: true, error: null });
      await Promise.all([get().fetchChannels(), get().fetchAppStreams()]);
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
}));
