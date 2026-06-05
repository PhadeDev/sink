# Sink

**A mixing board for your Linux desktop.** Every app gets its own channel —
route it, mix it, send it where you want. Built on PipeWire; think
SteelSeries Sonar or Voicemeeter, but native.

![The Sink mixer — capture, channels and mixes side by side](docs/mixer.png)

```
 apps ─► channels ─► your ears
              └────► a Mix ─► OBS / recorder
```

## What you get

🎚️ **Channels** — game, chat, music, whatever you like. Each has its own
volume, mute, live meters, and can play to a different output device
(or just follow your system default).

📦 **Automatic app routing** — running apps show up with their real names
and icons. Drop each one onto a channel once; Sink remembers and routes it
the same way next time.

![The Apps screen — every stream, routed and remembered](docs/apps.png)

🎙️ **A better mic** — a cleaned-up virtual microphone with a noise gate,
compressor and limiter between your hardware mic and the apps that hear
you. Pick it by name in Discord or OBS, and listen to yourself while you
dial it in.

![The Mic screen — gate, compressor and limiter with live tuning](docs/mic.png)

🎛️ **Mixes** — recordable copies of whatever channels you pick. The Master
Mix always carries everything; add a custom one for "everything except my
music" and it keeps itself up to date as you add channels. In OBS, add a
mix as an audio *input* (mic/aux) — not Desktop Audio.

💾 **Profiles** — save full layouts, switch them from the tray, and let a
profile load automatically when a particular output device is in use.

## Install

Grab the latest build from [Releases](https://github.com/NC1107/sink/releases):

- **AppImage** — portable; `chmod +x` and run
- **.deb** — Debian, Ubuntu and friends
- **.rpm** — Fedora, openSUSE and friends

You'll need PipeWire (with `pipewire-pulse`) and WirePlumber 0.5+ — already
the default on Fedora, Ubuntu 22.10+, Arch, and most modern distros.

## Build from source

```bash
npm install
npm run tauri dev      # run it
npm run tauri build    # package it (deb / rpm / AppImage)
```

Checks: `cargo test` and `cargo clippy` (from `src-tauri/`),
`npx tsc --noEmit`, `npm test`.

## Where things live

Everything is plain JSON under `~/.config/sink` — channels, mixes,
assignments, profiles, preferences. Settings → Reset Sink puts the app
back to factory state.

## License

[GPL-3.0](LICENSE)
