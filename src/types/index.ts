// Mirrors the Rust structs in src-tauri/src/audio/types.rs — keep in sync.

export interface AppStream {
  index: number;
  app_name: string;
  /** PipeWire property app_name was read from (stream identity for persistence). */
  match_prop: string;
  /** User-chosen display name overriding app_name. */
  alias: string | null;
  icon_name: string | null;
  /** Name of the virtual sink the stream is routed to, if any. */
  assigned_sink: string | null;
  volume_percent: number;
  muted: boolean;
  /** True while the stream is actively producing audio. */
  active: boolean;
}

export interface VirtualSink {
  /** e.g. "sink_game" */
  name: string;
  /** e.g. "Game" */
  label: string;
  volume_percent: number;
  muted: boolean;
}

export interface OutputDevice {
  index: number;
  name: string;
  description: string;
}

/** Phase 3 mic chain configuration (mirrors Rust MicConfig). */
export interface MicConfig {
  enabled: boolean;
  /** node.name of the hardware mic (null = system default). */
  input_device: string | null;
  /** 0–200; 100 = unity. */
  gain_percent: number;
  gate_enabled: boolean;
  comp_enabled: boolean;
  limiter_enabled: boolean;
  muted: boolean;
}

/** Profile listing entry (Phase 5: trigger_device auto-loads the profile). */
export interface ProfileInfo {
  name: string;
  trigger_device: string | null;
}

/** Sent as sink_name to unassign a stream (backend moves it to the default sink). */
export const UNASSIGNED = "";

export const MAX_VOLUME = 150;
export const MAX_MIC_GAIN = 200;
/** Levels key for the mic chain. */
export const MIC_LEVEL_KEY = "sink_mic";
