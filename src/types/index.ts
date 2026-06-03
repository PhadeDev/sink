// Mirrors the Rust structs in src-tauri/src/audio/types.rs — keep in sync.

export interface AppStream {
  index: number;
  app_name: string;
  icon_name: string | null;
  /** Name of the virtual sink the stream is routed to, if any. */
  assigned_sink: string | null;
  volume_percent: number;
  muted: boolean;
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

/** Sent as sink_name to unassign a stream (backend moves it to the default sink). */
export const UNASSIGNED = "";

export const MAX_VOLUME = 150;
