# Sink

SteelSeries Sonar for Linux. Built on PipeWire.

Route each app to its own channel — Game, Chat, Music — and control
volume, mute, and output device per channel. Build mixes for OBS and a
processed virtual microphone for voice chat.

![Mixer](docs/mixer.png)

```
 apps ─► channels ─► your ears
              └────► a Mix ─► OBS / recorder
```

## Features

- **Channels** — per-app routing with volume, mute, meters, and a choice
  of output device per channel
- **Apps** — running apps appear automatically; assign once, remembered
  forever
- **Mixes** — recordable sources for OBS. Master Mix carries everything;
  custom mixes can carry "everything except music" and stay current as
  channels change. In OBS, add a mix as an audio input — not Desktop Audio.
- **Microphone** — noise gate, compressor and limiter into a virtual mic
  you select in Discord or OBS
- **Profiles** — save and switch full layouts from the tray

![Mic](docs/mic.png)
![Apps](docs/apps.png)

## Install

Download from [Releases](https://github.com/NC1107/sink/releases):
AppImage (portable), .deb, or .rpm.

Requires PipeWire with `pipewire-pulse` and WirePlumber 0.5+ (the default
on most current distros).

## Build

```bash
npm install
npm run tauri dev      # run
npm run tauri build    # package
```

Config lives in `~/.config/sink` as plain JSON.

## License

[GPL-3.0](LICENSE)
