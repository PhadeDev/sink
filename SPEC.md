# Sink — Product Specification

## Overview

Sink is a Linux-native audio routing and mixing application. It presents a consumer-friendly mixer board UI for managing virtual audio channels, per-application routing, volume control, and mute — replacing the need for manual PipeWire/WirePlumber configuration.

The target user is a Linux gamer or streamer who wants the experience of SteelSeries Sonar or Voicemeeter without the Windows dependency.

---

## Name

`sink`

Lowercase. Named after PipeWire's concept of a sink (virtual audio output device).

---

## License

GPL-3.0

---

## Platform

**Linux-first.** Requires PipeWire and WirePlumber.

The Rust backend uses a swappable `AudioBackend` trait so macOS (CoreAudio) or Windows (WASAPI) backends can be added in future phases without UI changes.

---

## Tech Stack

| Layer | Technology |
|---|---|
| Backend language | Rust (edition 2021, MSRV 1.77) |
| Desktop framework | Tauri v2 |
| Frontend language | TypeScript |
| Frontend framework | React 18 |
| Styling | Tailwind CSS v4 |
| State management | Zustand |
| Audio backend (Phase 1) | `pactl` / `pw-dump` via subprocess calls |
| Audio backend (Phase 2+) | `pipewire-rs` native bindings |
| Persistence | JSON flat files |
| Config location | `$XDG_CONFIG_HOME/sink/` (default: `~/.config/sink/`) |
| Distribution | AUR (binary) + Flatpak |

---

## Architecture

```
┌────────────────────────────────────┐
│  Tauri/React UI                    │
│  Mixer board · App list · Profiles │
└────────────────┬───────────────────┘
                 │  Tauri IPC (invoke + events)
┌────────────────▼───────────────────┐
│  Rust Core                         │
│  ┌─────────────────────────────┐   │
│  │ AudioBackend trait          │   │
│  │  Phase 1: PactlBackend      │   │
│  │  Phase 2: PipeWireBackend   │   │
│  └─────────────────────────────┘   │
│  MixerState · ProfileStore         │
│  Tray icon                         │
└────────────────┬───────────────────┘
                 │
         ┌───────▼────────┐
         │   PipeWire      │
         │   WirePlumber   │
         └─────────────────┘
```

---

## Phases

### Phase 1 — Working Mixer (MVP)

**Goal:** A running app where you can see audio streams, assign them to channels, and control volume. No persistence.

Features:
- 4 named virtual sinks created at startup: **Game**, **Chat**, **Music**, **System**
- Virtual sinks destroyed cleanly on app exit
- Sidebar lists all active app audio streams (name + icon where available)
- Click an app stream to assign it to a channel
- Volume fader (0–150%) + mute toggle per channel
- Volume changes apply immediately via `pactl set-sink-volume`
- Window hides to system tray on close; tray menu has Show/Quit
- No persistence — state resets on close

Deliverable: installable binary that runs on any PipeWire distro.

---

### Phase 2 — Persistence

Features:
- WirePlumber Lua rules written to `~/.config/wireplumber/` when an app is assigned to a channel
- App-to-channel routing survives app restarts and reboots
- Save/load named profiles (e.g. Gaming, Streaming, Work)
- Profile stored as JSON in `$XDG_CONFIG_HOME/sink/profiles/`
- Swap `PactlBackend` for `PipeWireBackend` using `pipewire-rs`
- systemd user unit for autostart on login

---

### Phase 3 — Mic Channel

Features:
- 5th channel: **Mic** (virtual source/input)
- Built-in DSP chain applied to mic input: noise gate → gain → compressor → limiter
- No external LV2/LADSPA dependency — DSP implemented natively in Rust
- Visual input level meter in the UI
- Virtual monitor sink so apps like OBS can capture the processed mic

---

### Phase 4 — Output Routing

Features:
- Assign each channel to a specific physical output device independently
- Example: Game + Chat → headphones; Music → speakers
- Output device selector per channel in the UI

---

### Phase 5 — Polish

Features:
- OBS virtual audio source integration
- ChatMix-style Game/Chat balance slider (hardware knob support if device exposes it)
- Hardware device profiles that auto-switch on device connect/disconnect
- macOS CoreAudio backend if community demand warrants it

---

## UI Layout (Phase 1)

```
┌─────────────────────────────────────────────────────────┐
│  sink                                         [_]  [x]  │
├─────────────────────────────┬───────────────────────────┤
│  CHANNELS                   │  APPS                     │
│                             │                           │
│  GAME   CHAT   MUSIC   SYS  │  Firefox         → Game   │
│                             │  Discord         → Chat   │
│  ||||   ||||   ||||   ||||  │  Spotify         → Music  │
│                             │  Steam           → —      │
│  100%   85%    60%    100%  │                           │
│  [mute] [mute] [mute] [mute]│                           │
└─────────────────────────────┴───────────────────────────┘
```

Channels are vertical strip components with:
- Channel label (editable in Phase 2)
- VU meter (animated, live)
- Volume fader (vertical slider)
- Percentage readout
- Mute toggle button

App list shows:
- App name
- App icon (from desktop entry, fallback to generic)
- Current channel assignment (dropdown or click-to-assign)

---

## Error Handling

- If PipeWire is not running, show a clear error screen with instructions
- If `pactl` is not available, show install instructions
- Virtual sink creation failures are surfaced as toast notifications, not silent
- Backend errors are logged to `$XDG_STATE_HOME/sink/sink.log`

---

## File Structure (target)

```
sink/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs               # entry point
│   │   ├── lib.rs                # Tauri builder + handler registration
│   │   ├── error.rs              # SinkError enum
│   │   ├── state.rs              # AppState (managed by Tauri)
│   │   ├── audio/
│   │   │   ├── mod.rs
│   │   │   ├── backend.rs        # AudioBackend trait
│   │   │   ├── pactl.rs          # Phase 1: pactl subprocess impl
│   │   │   └── types.rs          # AppStream, SinkDevice, OutputDevice
│   │   ├── mixer/
│   │   │   ├── mod.rs
│   │   │   └── state.rs          # MixerState: channels, assignments, volumes
│   │   └── commands/
│   │       ├── mod.rs
│   │       ├── devices.rs        # Tauri commands: get/init/teardown devices
│   │       └── routing.rs        # Tauri commands: route, volume, mute
│   ├── tauri.conf.json
│   ├── Cargo.toml
│   └── build.rs
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── components/
│   │   ├── MixerBoard/           # Channel strips
│   │   ├── AppList/              # App stream list
│   │   └── TitleBar/             # Custom title bar
│   ├── types/
│   │   └── index.ts              # Shared TS types mirroring Rust structs
│   ├── store/
│   │   └── mixer.ts              # Zustand store
│   ├── hooks/
│   │   └── useAudio.ts           # PipeWire event listener hook
│   └── styles/
│       └── globals.css
├── SPEC.md                       # This file
├── AGENTS.md                     # Agent implementation brief
├── package.json
├── vite.config.ts
├── tsconfig.json
├── tailwind.config.ts
├── Cargo.toml                    # workspace root
└── LICENSE
```
