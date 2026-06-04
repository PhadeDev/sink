# Sink

A Linux-native audio routing and mixing app built on PipeWire — think
SteelSeries Sonar or Voicemeeter, but for the Linux desktop.

Create named virtual audio channels (Game, Chat, Music, System), assign
application audio streams to them, and control volume, mute, and output
routing per channel. Combine channels into capturable mixes for OBS and
other recorders, and publish a processed virtual microphone with a noise
gate, compressor, and limiter.

## Features

- **Channels** — user-defined virtual sinks with per-channel volume, mute,
  live VU metering, and per-channel output device routing (follow-default
  with automatic failover, or pin to a specific device)
- **App routing** — running streams are detected automatically, with
  desktop-entry icons and names; assignments persist and re-apply when an
  app reappears, including an app history with ignore/forget
- **Mixes** — an always-on Master Mix carrying every channel, plus up to
  four custom mixes (e.g. everything-but-music) exposed as capturable
  sources recorders can select by name
- **Microphone chain** — hardware capture → noise gate → gain → compressor
  → limiter → a virtual microphone other apps can use; tunable thresholds,
  renameable, with listen-to-yourself monitoring
- **Profiles** — save and switch full layouts (channels, assignments,
  outputs, mixes) from the app or the tray, with live autosave and
  per-output-device auto-switching
- **System integration** — tray icon with profile switching, optional
  autostart (systemd user unit), system default device pickers, and
  WirePlumber rules so routing survives outside the app

## Requirements

- Linux with PipeWire (with `pipewire-pulse`) and WirePlumber 0.5+
- `pactl` available on `$PATH` (used by the fallback backend)

Sink talks to PipeWire natively (via `pipewire-rs`); if the native
backend can't come up it falls back to `pactl`, with mixes, the mic
chain, and live metering disabled.

## Building

Standard Tauri v2 + Vite project:

```bash
npm install              # frontend deps
npm run tauri dev        # run in dev mode
npm run tauri build      # production build (deb, rpm, appimage)
```

Tests and checks:

```bash
cargo test               # Rust tests (from src-tauri/)
cargo clippy             # Rust lint (from src-tauri/)
npx tsc --noEmit         # TypeScript type check
npm test                 # frontend tests (Vitest)
```

## Configuration

Everything lives in `$XDG_CONFIG_HOME/sink` (usually `~/.config/sink`):
channels, mixes, assignments, profiles, app history, and preferences —
all plain JSON. Settings → Reset Sink restores a factory-fresh state.

## License

GPL-3.0 — see [LICENSE](LICENSE).
