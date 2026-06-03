# Sink — Agent Implementation Brief

## What You Are Building

Sink is a Linux-native audio routing and mixing application — the missing Voicemeeter/SteelSeries Sonar equivalent for Linux.

It lets users create named virtual audio channels (Game, Chat, Music, System), assign running applications to those channels, and control volume/mute per channel — all through a clean desktop UI backed by PipeWire.

Read SPEC.md for full context, architecture, and all phases before writing any code.

---

## Your Task: Implement Phase 1

Phase 1 is the MVP. When complete, a user should be able to:

1. Launch Sink
2. See 4 virtual channels created automatically (Game, Chat, Music, System)
3. See a list of all running app audio streams
4. Click an app and assign it to a channel
5. Use a volume fader and mute button per channel
6. Close the window — app hides to tray
7. Quit from the tray icon — virtual sinks are cleaned up

**No persistence in Phase 1.** State resets on close.

---

## Rules

- Do not implement Phase 2, 3, 4, or 5 features
- Do not add features not listed in Phase 1 of SPEC.md
- Do not use Electron — use Tauri v2
- Do not use PulseAudio-only APIs — use `pactl` which works on both PulseAudio and PipeWire-pulse
- Do not hardcode paths — use `$XDG_CONFIG_HOME` or `dirs` crate for config paths
- Do not panic — surface errors to the UI via Tauri events or command return values
- Follow the file structure defined in SPEC.md exactly
- Backend must implement the `AudioBackend` trait even in Phase 1 — do not call `pactl` directly from commands
- All Tauri commands return `Result<T, String>` — errors are strings for serialisation

---

## Tech Stack (exact versions)

| | |
|---|---|
| Rust | edition 2021, MSRV 1.77 |
| Tauri | v2 |
| React | 18 |
| TypeScript | 5 |
| Tailwind CSS | v4 |
| Zustand | latest |
| Build tool | Vite |

---

## Backend: AudioBackend Trait

Define this trait in `src-tauri/src/audio/backend.rs`:

```rust
pub trait AudioBackend: Send + Sync {
    fn create_virtual_sink(&self, name: &str) -> Result<(), SinkError>;
    fn destroy_virtual_sink(&self, name: &str) -> Result<(), SinkError>;
    fn list_app_streams(&self) -> Result<Vec<AppStream>, SinkError>;
    fn list_output_devices(&self) -> Result<Vec<OutputDevice>, SinkError>;
    fn set_sink_volume(&self, sink_name: &str, volume_percent: u8) -> Result<(), SinkError>;
    fn set_sink_mute(&self, sink_name: &str, muted: bool) -> Result<(), SinkError>;
    fn move_stream_to_sink(&self, stream_index: u32, sink_name: &str) -> Result<(), SinkError>;
}
```

Phase 1 implementation is `PactlBackend` in `src-tauri/src/audio/pactl.rs`. It calls `pactl` as a subprocess. Use `std::process::Command`. Parse `pactl list sink-inputs` output to get app streams.

---

## Shared Types

Define in `src-tauri/src/audio/types.rs` and mirror in `src/types/index.ts`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStream {
    pub index: u32,
    pub app_name: String,
    pub icon_name: Option<String>,
    pub assigned_sink: Option<String>,
    pub volume_percent: u8,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualSink {
    pub name: String,       // e.g. "sink_game"
    pub label: String,      // e.g. "Game"
    pub volume_percent: u8,
    pub muted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputDevice {
    pub index: u32,
    pub name: String,
    pub description: String,
}
```

---

## Tauri Commands to Implement

In `src-tauri/src/commands/`:

**devices.rs**
- `get_virtual_devices() -> Result<Vec<VirtualSink>, String>`
- `get_app_streams() -> Result<Vec<AppStream>, String>`
- `get_output_devices() -> Result<Vec<OutputDevice>, String>`
- `init_virtual_devices() -> Result<(), String>` — creates the 4 default sinks
- `teardown_virtual_devices() -> Result<(), String>` — destroys them on quit

**routing.rs**
- `route_app_to_channel(stream_index: u32, sink_name: String) -> Result<(), String>`
- `set_channel_volume(sink_name: String, volume: u8) -> Result<(), String>`
- `toggle_channel_mute(sink_name: String, muted: bool) -> Result<(), String>`
- `set_app_volume(stream_index: u32, volume: u8) -> Result<(), String>`

---

## Virtual Sink Names

Use these exact internal names and labels:

| Internal name | Display label |
|---|---|
| `sink_game` | Game |
| `sink_chat` | Chat |
| `sink_music` | Music |
| `sink_system` | System |

Create them with:
```bash
pactl load-module module-null-sink sink_name=sink_game sink_properties=device.description=Game
```

Destroy them with:
```bash
pactl unload-module <module_index>
```

Track module indices in `MixerState` so you can unload the right module on exit.

---

## Frontend

### Layout

Two-panel layout. Left panel: channel strips. Right panel: app list.

No third-party component libraries. Build components from scratch with Tailwind.

### Channel Strip Component (`MixerBoard/ChannelStrip.tsx`)

- Display label (e.g. "Game")
- VU meter (animated bars, driven by polling or Tauri events — polling is fine for Phase 1)
- Vertical volume slider (range 0–150)
- Volume percentage display
- Mute toggle button (changes color when muted)

### App List Component (`AppList/AppRow.tsx`)

- App name
- App icon (use `icon_name` to look up from `/usr/share/icons/` or fallback SVG)
- Channel assignment selector (dropdown showing the 4 channels + unassigned)

### Zustand Store (`store/mixer.ts`)

Store shape:
```ts
interface MixerStore {
  channels: VirtualSink[]
  appStreams: AppStream[]
  fetchChannels: () => Promise<void>
  fetchAppStreams: () => Promise<void>
  setChannelVolume: (sinkName: string, volume: number) => Promise<void>
  toggleMute: (sinkName: string, muted: boolean) => Promise<void>
  routeApp: (streamIndex: number, sinkName: string) => Promise<void>
}
```

Poll `get_app_streams` every 2 seconds to detect new/closed apps. Use `setInterval` in a `useEffect` in App.tsx.

### Window Behaviour

- Custom frameless title bar with window drag region
- Close button hides to tray (intercept `tauri://close-requested` event)
- Tray menu: Show Window / Quit

On Quit: call `teardown_virtual_devices` before exit.

---

## Acceptance Criteria for Phase 1

- [ ] App launches without errors on a PipeWire system
- [ ] 4 virtual sinks appear in `pactl list sinks` after launch
- [ ] App audio streams are listed in the UI
- [ ] Assigning an app to a channel moves it: verify with `pactl list sink-inputs`
- [ ] Volume fader changes are reflected immediately in `pactl list sinks`
- [ ] Mute toggle is reflected immediately
- [ ] Closing the window hides to tray; app is still running
- [ ] Quit from tray removes all 4 virtual sinks from `pactl list sinks`
- [ ] No panics or unwrap failures in normal usage

---

## What NOT to Do

- Do not implement WirePlumber rule writing (Phase 2)
- Do not implement profiles or persistence (Phase 2)
- Do not implement mic DSP chain (Phase 3)
- Do not implement per-channel output routing (Phase 4)
- Do not add a settings screen
- Do not use `unsafe` Rust unless absolutely required by a C FFI
- Do not use `unwrap()` or `expect()` in production paths — propagate errors
