# Technical Debt

Last updated: 2026-07-01 (deep multi-domain audit, branch `dev-pipeline`).

Method: six parallel domain reviews (security, Rust backend, frontend, performance, test quality, DevOps/packaging), followed by synthesis, cross-agent deduplication, and spot verification of the highest-severity claims against the working tree.
Every finding cites the file and line where it was verified.
No code was changed as part of this audit; this document is the only artifact.

## Summary

| Severity | Count |
|----------|-------|
| Critical | 0 |
| High | 10 |
| Medium | 37 |
| Low | 30 |
| **Total** | **77** |

Primary areas: Rust backend 15, DevOps/packaging 14, performance 14, frontend 14, testing 12, security 8 (several findings span two areas and are merged below).

## What is healthy (calibration)

The audit found no critical issues and a lot done right; the findings below should be read against this baseline.

- The `AudioBackend` trait boundary is genuinely respected; no Tauri command bypasses it, and the pactl and native backends are cleanly interchangeable.
- Zero `unsafe`; exactly one production-path `expect()` in the whole backend (TD-005 area, see TD-050 note); volume inputs are clamped.
- Tauri IPC surface is deliberately small: no `fs`/`shell`/`http` plugins, CSP is `script-src 'self'`, asset-protocol scope is restricted to icon directories, config dir is 0700, profile names are traversal-sanitized and unit-tested.
- The repo has 65 passing tests (56 Rust across 18 files, 9 Vitest), all meaningful; PR CI runs `tsc --noEmit`, Vitest, `clippy -D warnings`, and `cargo test`.
- Rust-to-TS type sync was checked field by field across all eight shared shapes: currently zero drift.
- The realtime core is allocation-free in the DSP/meter callbacks, levels move through a lock-free store, and steady-state link management is fully event-driven.
- No auto-updater exists, so there is no runtime update-fetch attack surface; the Arch PKGBUILD pins the release artifact by SHA-256 and encodes the appindicator dependency lesson.

---

## High

### TD-001: Package smoke test only detects Rust panics, not loader or init failures
`.github/workflows/dev.yml:122-123` (same logic `.github/workflows/release.yml:172`) - devops/release-validation - defect - effort: small
The launch gate is `timeout 25 xvfb-run -a /usr/bin/sink ... || true` followed by `grep -q 'panicked at'`.
The `|| true` swallows every exit code, so a missing shared library ("error while loading shared libraries"), a GTK/webkit init abort, or an instant clean exit all pass and the broken package publishes.
This is the same failure class as the shipped appindicator incident; the current check would not catch its non-panic variants.
Fix: require the process to survive until `timeout` kills it (exit code 124) and additionally grep for `error while loading shared libraries`.

### TD-002: .deb and .rpm declare no explicit runtime dependencies
`src-tauri/tauri.conf.json:41-50` - devops/packaging - defect - effort: medium
The bundle block has no `linux.deb.depends` or `linux.rpm.depends` (verified: no `linux` key at all).
Soname auto-detection cannot capture `pactl` (executed, not linked; the entire fallback backend needs it), `pipewire-pulse`/`wireplumber` (required session services), or `libayatana-appindicator` (dlopen'd, invisible to the scanner).
The Arch PKGBUILD already fixed exactly this; deb/rpm did not get the same fix, so a minimal Debian/Fedora install can get a package that launches broken.
Fix: add explicit depends per format and extend the launch smoke test to deb/rpm in clean containers.

### TD-003: Externally destroyed virtual mic is never healed
`src-tauri/src/audio/pw_native/thread.rs:252` - rust/logic - defect - effort: small
The pre-heal cleanup drops dangling proxies for kinds 0/1 but the kind-2 arm is `_ => {}`, so `mic_source` is never cleared (verified).
The stale proxy makes `already_back` true, the heal takes `Heal::Relink`, and `sink_mic` is never recreated after wpctl or a session-manager hiccup destroys it.
Discord/OBS lose the mic until the user toggles the chain off and on, breaking the documented self-healing guarantee for exactly this node.
Fix: clear `s.mic_source` in the cleanup match (guarding the deliberate mic-rename recreate).

### TD-004: Mixer mutex held across blocking backend I/O in the 2-second poll path
`src-tauri/src/commands/devices.rs:41` - rust/concurrency - defect - effort: small
`get_app_streams` takes `lock_mixer()` and, while holding it, performs disk I/O (`mixer.seen.save()`, line 61) and backend calls (`move_stream_to_sink`, line 77) that block up to the 3s native request timeout each (or a subprocess spawn on pactl).
If the PipeWire loop is slow or dead, one poll holds the lock for 3s x N streams while every other command, including tray menu building, blocks behind it; the poll keeps stacking new blocked invocations.
Fix: snapshot routing decisions under the lock, drop the guard, do the backend calls and save outside it, then re-lock briefly to record results.

### TD-005: Non-atomic config writes plus silent default fallback can destroy user configuration
`src-tauri/src/persistence/channels.rs:102` (pattern repeated in all 11 persistence modules) - rust/persistence - defect - effort: small
Every save uses plain `fs::write`; a crash or power loss mid-write leaves truncated JSON.
Load paths then silently discard it: `Channels::load` does `.ok().filter(...).unwrap_or_default()` with no log, resetting the user's whole channel set to the default four.
`seen.save()` runs from the 2s poll, making torn writes most likely on that file; worst case is `wireplumber.rs:74`, where a truncated conf.d fragment is malformed SPA-JSON that can break WirePlumber config parsing at next login.
Fix: one shared temp-file-plus-rename helper in `persistence::mod` (with `sync_all`), and log or surface whenever a config file is discarded as malformed.

### TD-006: Popover focus trap re-runs on unstable `onClose`, yanking keyboard focus ~10x/s while audio plays
`src/components/Popover.tsx:109` (same pattern `src/components/Modal.tsx:23`) - frontend/accessibility - defect - effort: small
The focus-trap effect deps are `[open, onClose]` (verified) and every call site passes an inline arrow, while ChannelStrip re-renders at 10 Hz whenever any channel has signal.
Each re-render tears down and re-runs the trap, calling `previous?.focus?.()` then `menuRef.current?.focus()`, so with a popover open during playback focus is forcibly reset up to 10 times per second and tabbing to a menu item is impossible.
Fix: hold `onClose` in a ref updated each render and key both effects on `[open]` only.

### TD-007: Mix strip volume/mute is throwaway local state that desyncs from the backend
`src/components/MixerBoard/StreamMixStrip.tsx:29-30` - frontend/state - defect - effort: medium
`useState(100)` / `useState(false)` are never hydrated from the backend (verified; `BusDef` carries no volume).
Set a mix to 30%, switch tabs and back: the fader shows 100% while OBS still records at 30%, and the first fader touch jumps the real recording level.
Fix: keep bus volume/mute in the Zustand store keyed by bus name, and report actual levels from `list_buses` so the UI hydrates from truth.

### TD-008: Level emitter streams IPC events at 10 Hz forever, even hidden in tray at total silence
`src-tauri/src/lib.rs:128-141` - performance/idle-churn - improvement - effort: small
`spawn_level_emitter` loops unconditionally every 100 ms: locks the slot registry, clones every meter name, builds and serializes a HashMap, and emits into the webview - 864k emissions/day with no all-zero suppression, no visibility gating, no subscriber check.
The frontend suppresses no-op updates only after the IPC wake and JSON parse already happened.
Fix: skip `emit` when all values stay below epsilon, and gate on window visibility (or a frontend subscribe/unsubscribe command).

### TD-009: The 2-second poll (4 IPC commands including the full app history) never pauses while hidden
`src/hooks/useAudio.ts:23-31` - performance/idle-churn - improvement - effort: small
The interval fires `fetchAppStreams` plus `fetchOutputs` (two invokes) plus `fetchSeenApps` - 4 IPC round-trips every 2s, 172,800/day - and nothing checks visibility, while the close button only hides the window.
The product's dominant state (sitting in the tray during a game) runs the full poll forever, including serializing the entire ever-seen app history each tick.
Fix: pause or stretch the interval when hidden and refresh on show; longer term move auto-route enforcement backend-side so hiding the UI cannot disable routing.

### TD-010: The failover/link-reconcile logic (largest file in the repo) has zero tests and is partly untestable
`src-tauri/src/audio/pw_native/thread.rs:635` and `:708` - testing/coverage - defect+improvement - effort: medium
`desired_pairs` (channel matching, mono fan-out, index-wrap fallback) is pure and unit-testable today with `State::default()`, yet has no tests; a regression means swapped stereo or one-ear audio.
`ensure_all_links` (output failover, bus gating, monitor suppression) is interleaved with the `CORE` thread-local and link side effects, so the headline Sonar-style failover behavior is verified only by hand (see also TD-017).
Fix: add `desired_pairs` unit tests now; split reconcile into a pure `plan_links(&State) -> LinkPlan` plus a thin applier, then unit-test the plan (failover, no-churn, monitor suppression).

---

## Medium

### TD-011: Init-timeout race leaves a zombie PipeWire loop fighting the pactl fallback
`src-tauri/src/audio/pw_native/mod.rs:54` - rust/lifecycle - defect - effort: medium
If init times out (5s), the app falls back to `PactlBackend` but the spawned loop thread is never stopped; nothing calls `mainloop.quit()` and there is no `Drop`.
A late-arriving loop then adopts the pactl-created sinks and creates monitor links while the pactl backend creates loopbacks for the same channels: every channel plays twice, plus a leaked thread.
Fix: signal shutdown on the timeout path, or require a "go" confirmation before the thread enters `run()`.

### TD-012: No PipeWire disconnect detection; backend silently serves stale state after a daemon restart
`src-tauri/src/audio/pw_native/thread.rs:197` - rust/reliability - defect - effort: large
No core error/disconnect listener exists, so after a daemon restart the loop keeps replying `Ok` from the stale `State` mirror and healing never fires.
The app appears healthy while doing nothing, with no recovery path and no error surfaced (the pactl backend, by contrast, maps connection failures to `ServerUnreachable`).
Fix: register a core listener; on error either reconnect-and-rebuild or flip a poisoned flag so `handle_cmd` replies `ServerUnreachable`.

### TD-013: `channels.json` is loaded without invariant validation
`src-tauri/src/persistence/channels.rs:82` - rust/validation - defect - effort: small
`load()` only checks non-empty, bypassing every invariant `add()` enforces: a hand-edited `sink_mic` channel collides with the mic service node, a non-`sink_`-prefixed name makes `init_virtual_devices` abort on the first channel, and more than 10 channels exhausts the 12-slot meter budget.
Fix: drop reserved/unprefixed/duplicate entries and truncate to `MAX_CHANNELS` in `load()`, logging what was discarded.

### TD-014: Partial init failures are swallowed to stderr; channels can be silent with no UI error
`src-tauri/src/commands/devices.rs:158` (also 147-149, 166, 173, 183; same pattern in `commands/profiles.rs`) - rust/error-handling - defect - effort: small
Output wiring, bus creation, bus membership, and mic bring-up are best-effort `eprintln!`; if `set_channel_output` fails the channel produces no audio at all, yet the command returns `Ok(())` and the UI renders a healthy mixer.
Fix: collect non-fatal failures into the command result or emit a `backend-warning` event the frontend surfaces.

### TD-015: Channel volume/mute has two sources of truth; external changes diverge and get autosaved stale
`src-tauri/src/commands/devices.rs:9` - rust/architecture - improvement - effort: medium
`get_virtual_devices` serves `MixerState.channels` (UI-written) while the native thread already tracks real per-node volume/mute from Props events.
A pavucontrol/wpctl change leaves the UI stale, and `autosave_active` then persists the stale values into the active profile on the next unrelated mutation.
Fix: make the backend authoritative for volume/mute and merge live values over persisted labels/icons.

### TD-016: `add_channel` rollback leaves the created sink alive and invisible
`src-tauri/src/commands/channels.rs:33` - rust/lifecycle - defect - effort: small
If a later step fails after `create_virtual_sink` succeeds, only the definition is rolled back; natively the sink stays in `owned_sinks`/`desired` and is actively healed all session while absent from the UI and from teardown.
Fix: call `destroy_virtual_sink` in the error branch.

### TD-017: Follow-default channels have no independent fallback when the default device disappears
`src-tauri/src/audio/pw_native/thread.rs:744-751` - rust/routing - improvement - effort: medium
A follow-default channel's target is strictly WirePlumber's `default.audio.sink` resolved against live nodes; if that node is gone and WirePlumber does not reassign (wireless dongle still present, WP lagging or misconfigured), `target_id` is `None`, the channel drops to zero links, and output is silence.
Field report (2026-06, Arctis Nova 7 on CachyOS): headset off yields no audio on speakers because nothing reassigns the default; Sink correctly follows once WP moves it, but has no priority-based fallback of its own despite CLAUDE.md advertising Sonar-style auto-failover.
Fix: when the resolved default is absent, fall back to the highest-priority available hardware sink until the metadata catches up (and relink when it does).

### TD-018: Incomplete escaping enables SPA-JSON injection into WirePlumber default-device metadata
`src-tauri/src/audio/pw_native/thread.rs:1247` - security/injection (also rust/correctness) - defect - effort: small
The metadata value is hand-built with `format!`, escaping `"` but not backslash; a name ending in `\` breaks out of the quoted string, and crafted names can inject keys into the `Spa:String:JSON` value WirePlumber consumes.
Reachable from a compromised webview via `set_default_output`/`set_default_input`; inconsistent with the codebase's own correct escaper in `persistence/wireplumber.rs:29`.
Fix: `serde_json::json!({"name": name}).to_string()`.

### TD-019: Untrusted `application.icon-name` becomes an arbitrary asset-protocol path
`src-tauri/src/audio/icons.rs:232` - security/path-traversal - defect - effort: small
`icon_name_to_path` accepts an absolute path verbatim if it exists, and the first candidate is the stream's attacker-controlled `application.icon-name`; the result feeds `convertFileSrc` and the asset protocol.
The asset-scope allowlist is the only remaining guard and should be defense-in-depth, not the fix.
Fix: reject absolute paths and `..` components from the icon hint; canonicalize and verify the result is inside a known icon root.

### TD-020: Third-party GitHub Actions pinned to mutable refs in jobs holding `contents: write`
`.github/workflows/release.yml:40,42` (also dev.yml:23,27) - security/supply-chain (also devops) - improvement - effort: small
`dtolnay/rust-toolchain@stable` is literally a moving branch and `swatinem/rust-cache@v2` a mutable tag; a compromised upstream runs inside the release job and can tamper with published artifacts.
`container: archlinux:latest` is likewise unpinned.
Fix: pin third-party actions to full commit SHAs with version comments, bumped by Dependabot (TD-065).

### TD-021: `workflow_dispatch` version input interpolated into a shell run block
`.github/workflows/release.yml:72` - devops/security - defect - effort: small
`input="${{ github.event.inputs.version }}"` splices the dispatch input directly into shell in a `contents: write` job; a crafted value executes arbitrary commands.
Exposure is limited to users who can already dispatch workflows, but the archpkg job already does this correctly via `env`.
Fix: bind through `env:` and reference `"$INPUT_VERSION"`.

### TD-022: Stable release flow has no version validation and no branch guard
`.github/workflows/release.yml:73-74` - devops/release-correctness - defect - effort: small
Any non-empty input is accepted verbatim (`v0.1.20` input yields tag `vv0.1.20`), nothing checks the tag does not already exist, and dispatching from any branch tags that SHA, so a stable release can be cut from an unreviewed feature branch.
Fix: regex-validate semver, fail on existing tags, and require `github.ref == refs/heads/main` for stable dispatches.

### TD-023: dev.yml has no `permissions` block
`.github/workflows/dev.yml:1-14` - devops/security - improvement - effort: small
release.yml scopes both jobs to `contents: write` but dev.yml gets the repository default token scope; a compromised dependency running during `npm ci`/`cargo test` on a PR could push content.
Fix: add workflow-level `permissions: contents: read`.

### TD-024: No checksums or signatures on published release artifacts; `--clobber` allows silent replacement
`.github/workflows/release.yml:133-136` - devops/supply-chain - improvement - effort: small
Artifacts upload bare; users have no integrity verification, and a re-run or leaked write token can silently replace a stable artifact under the same tag.
Fix: generate and upload `SHA256SUMS` (optionally minisign it) and drop `--clobber` for the stable path.

### TD-025: `Cargo.toml` is never version-stamped; all three manifests frozen at 0.1.0
`.github/workflows/release.yml:86-92` - devops/versioning - defect - effort: small
The stamp step rewrites tauri.conf.json and package.json but not `src-tauri/Cargo.toml`, so anything reading `CARGO_PKG_VERSION` reports 0.1.0 and checked-in files permanently lie about the released version (v0.1.19).
Fix: stamp all three, or make Cargo.toml the single source of truth and let tauri.conf.json inherit it.

### TD-026: rust-cache points at the wrong workspace, so Rust build caching is dead
`.github/workflows/dev.yml:27-29` (also dev.yml:61-63, release.yml:42-44) - devops/ci-efficiency - defect - effort: small
The Cargo workspace root is the repo root (verified: root `Cargo.lock`, no `src-tauri/Cargo.lock`), but `workspaces: src-tauri` caches a `src-tauri/target` that never exists and keys off lockfiles that are not there.
Every CI run recompiles the full Tauri dependency tree across three jobs, easily 20+ wasted minutes per run.
Fix: `workspaces: .` (or drop the option).

### TD-027: Release workflow publishes without running any tests
`.github/workflows/release.yml:3` - testing/ci (also devops) - defect - effort: small
release.yml contains no `cargo test`, `npm test`, `tsc`, or clippy step, and dev.yml only fires on `pull_request`, so a push to main or a dispatch builds and publishes bundles from code that never passed the suite.
Fix: add the four check steps (or a reusable job shared with dev.yml) as a prerequisite of the release build.

### TD-028: pactl fallback forks 3 subprocesses per poll tick (~130k/day)
`src-tauri/src/audio/pactl.rs:206-214` (and 253) - performance/subprocess-churn - improvement - effort: medium
`list_app_streams` runs two pactl processes per call and the same tick's `get_output_devices` runs a third, each a fork+exec plus fresh PulseAudio connection, roughly 1-2% of a core continuously in fallback mode.
Fix: cache the sink map with invalidation (or a persistent `pactl subscribe` child) and share one `list_sinks` result per tick.

### TD-029: Every poll tick replaces store arrays wholesale, re-rendering the whole board with zero changes
`src/store/mixer.ts:258-265` (also 329-339, 425-432; consumers MixerBoard.tsx:59-60, OutputSelect.tsx:28) - performance/render-churn (also frontend) - defect - effort: small
`fetchAppStreams`/`fetchOutputs`/`fetchSeenApps` call `set` unconditionally with fresh arrays, so identity changes every 2s even when content is byte-identical and every subscriber (all strips, AppList, ProfileMenu) reconciles 43,200 times/day at idle.
Fix: structural-equality guard before `set`, mirroring the existing `setLevels` pattern.

### TD-030: Levels identity churn re-renders every strip at 10 Hz while any audio plays
`src/store/mixer.ts:131-144` (consumers ChannelStrip.tsx:34, StreamMixStrip.tsx:25, MicStrip.tsx:14) - performance/render-churn (also frontend) - defect - effort: small
`setLevels` skips only the all-unchanged case; one playing channel replaces the whole `levels` object with fresh `[l, r]` arrays for silent channels too, so every strip fails `Object.is` and re-renders 10x/s all day during gameplay.
The rAF meter animation is correctly outside React; the React commits around it are pure waste.
Fix: per-key merge reusing previous tuple references when values are within epsilon.

### TD-031: Per-strip 60 Hz rAF loops never park, even at silence
`src/components/MixerBoard/VuMeter.tsx:34-66` (same pattern Mic/MicScreen.tsx:19-30) - performance/animation (also frontend) - improvement - effort: small
Each meter runs an unconditional `requestAnimationFrame` loop writing `clipPath`/`bottom`/`className` every frame even when fully decayed - hundreds of callbacks and style writes per second while visible but idle.
Fix: stop the loop when target, smooth, and peak are all below epsilon and restart it when a level arrives.

### TD-032: `auto_routed` grows unbounded and PipeWire ID recycling makes stale entries wrong
`src-tauri/src/mixer/state.rs:37` (inserted at commands/devices.rs:86) - performance/memory (correctness) - defect - effort: small
Entries are only cleared on `load_profile`, never when a stream disappears; PipeWire recycles global IDs, so a dead stream's ID reused by a new stream is silently skipped by auto-routing, and the likelihood grows with uptime.
Fix: `auto_routed.retain(|i| live.contains(i))` at the end of each `get_app_streams` pass.

### TD-033: Seen-apps history grows forever and is fully re-shipped every 2 seconds
`src-tauri/src/persistence/seen.rs:103-112` (consumed by commands/apps.rs:26-63) - performance/memory - defect - effort: medium
The list is append-only (manual forget aside): months of browser tab titles and one-off tools accumulate, get cloned, icon-resolved, linearly scanned, and serialized across IPC every tick.
Fix: age-prune unassigned entries on load/save and fetch seen-apps on demand instead of on the 2s poll.

### TD-034: Every settled volume/mute/output change does a profile read+parse+rewrite, non-atomically
`src-tauri/src/commands/profiles.rs:12-29` (called from routing.rs:76,96; devices.rs:258) - performance/disk-io - improvement - effort: medium
`autosave_active` re-loads the profile from disk just to preserve `trigger_device`, then rewrites the whole profile per settle; wiggling a fader produces a read+write pair each time, and `fs::write` is non-atomic (see TD-005).
Fix: cache `trigger_device` in `MixerState`, debounce the write backend-side, write via temp+rename.

### TD-035: pactl-fallback startup issues ~24+ serialized subprocess calls before the UI is usable
`src-tauri/src/commands/devices.rs:113-175` - performance/startup - improvement - effort: medium
Per channel: idempotency `list_sinks` fork, `load-module`, volume fork, mute fork, then two more for output wiring - roughly 250-500 ms of serial round-trips for the default four channels, with `list_sinks` re-run from scratch each time.
Fix: fetch `list_sinks` once per init and reuse the snapshot; skip volume/mute resets for freshly created sinks.

### TD-036: A successful poll wipes any action error; the global error banner self-dismisses in under 2 seconds
`src/store/mixer.ts:261` - frontend/error-handling - defect - effort: small
`fetchAppStreams` sets `error: null` on success every 2s, erasing every failed-action report before the user can read it, and the same field drives the TitleBar engine status, conflating fatal and one-off failures.
Fix: split `engineError` (owned by init/polling) from `actionError` (user-dismissed or fixed timeout).

### TD-037: Mix fader fires one `invoke` per pointer-move with all errors swallowed
`src/components/MixerBoard/StreamMixStrip.tsx:41-49` - frontend/async - defect - effort: small
`applyVolume` bypasses the store's `debouncedInvoke` (whose comment exists precisely to avoid a subprocess per pixel), and all four invokes in the file end in `.catch(() => {})`, so failures leave the optimistic UI lying silently.
Fix: store actions `setBusVolume`/`toggleBusMute` using `debouncedInvoke` with the standard error handler.

### TD-038: The 2s poll overwrites in-progress optimistic app-volume drags
`src/store/mixer.ts:315-327` with `src/hooks/useAudio.ts:25-29` - frontend/state - defect - effort: small
A poll response landing mid-drag (built from backend state predating the pending debounced write) snaps the HSlider knob back to the stale value, visible jitter on any drag longer than ~2s.
Fix: in `fetchAppStreams`, preserve local values for streams with a pending entry in `pendingInvokes`.

### TD-039: Modal and OnboardingModal have no focus containment; Onboarding ignores Escape
`src/components/Modal.tsx:26-37`; `src/components/Onboarding/OnboardingModal.tsx:90-91` - frontend/accessibility - defect - effort: medium
`Modal` renders `role="dialog"` without `aria-modal`, does not trap Tab, and never restores focus; keyboard users can tab out of "Delete channel?" into obscured controls.
OnboardingModal has no keyboard handling at all, making first-run mouse-only; Popover already implements the correct trap, so this is internal inconsistency.
Fix: extract Popover's trap into a shared hook and apply it to both, with `aria-modal="true"` and focus restore.

### TD-040: Toggle renders an empty button with no accessible name
`src/components/Toggle.tsx:5` - frontend/accessibility - defect - effort: small
The switch is a childless `<button>` with only `aria-pressed`, gating the mic chain, autostart, and start-minimized; screen readers announce "toggle button, pressed" with zero context.
Fix: required `label` prop rendered as `aria-label` (ToggleRow passes its title); ideally `role="switch"` + `aria-checked`.

### TD-041: PactlBackend has no subprocess seam; error classification and stream mapping are untested
`src-tauri/src/audio/pactl.rs:77` - testing/testability - improvement - effort: medium
`run()` calls `Command::new("pactl")` as a static fn, so the `ServerUnreachable` stderr sniffing, `load-module` stdout parse, and the full `list_app_streams` mapping (corked inversion, `assigned_sink` filtering, identity resolution) are all unverifiable against version-variant pactl output.
Fix: extract `classify_failure` and `map_streams(sinks, inputs)` and test with captured real-world JSON fixtures.

### TD-042: Stale-loopback cleanup scrapes human-oriented pactl text with zero tests
`src-tauri/src/audio/pactl.rs:382-392` (also 122-135) - testing/unit-gap - defect - effort: small
Despite the module doc claiming structural parsing, this path line-scans `list modules short` with `contains(&needle)`; a false negative stacks duplicate loopbacks (audio plays twice), and prefix-colliding sink names (`sink_game` vs `sink_game_2`) make `contains` matching dangerous.
Fix: extract `stale_loopback_indices(listing, sink_name)` and fixture-test it, including prefix collisions.

### TD-043: No back-compat tests for old config JSON shapes
`src-tauri/src/persistence/channels.rs:87` (also profiles.rs:13-28, types.rs:283-302) - testing/compatibility - defect - effort: small
Migration rests entirely on untested serde defaults; not one test loads a pre-Phase-4 JSON document to prove an upgrade keeps user data, and the silent reset-to-defaults behavior (TD-005) is not even pinned by a test.
Fix: committed fixtures for each older on-disk shape plus a `parse(&str)` seam like `Prefs` already has.

### TD-044: Command layer has zero tests; the auto-route once-only state machine runs every 2s untested
`src-tauri/src/commands/devices.rs:67-88` - testing/unit-gap - defect - effort: medium
The enforced-once contract ("a user moving a stream elsewhere isn't fought every poll") and input validation in routing.rs have no coverage; `AppState::new` eagerly reads the developer's real config dir, blocking test construction.
Fix: an `AppState`/`MixerState` test constructor taking pre-built persistence structs plus a recording `MockBackend` over the existing trait seam.

### TD-045: Rust-to-TS type sync is enforced by a comment, not by anything executable
`src/types/index.ts:1` - testing/contract - improvement - effort: medium
Eight interfaces are hand-mirrored; a serde rename compiles cleanly on both sides and fails only at runtime as `undefined`.
Currently in sync (verified field by field), but nothing keeps it that way.
Fix: `ts-rs`/`specta` codegen, or cheaper: Rust tests serialize one populated instance of each shared type to JSON fixtures that a Vitest test asserts key-by-key.

### TD-046: Frontend bus complement math duplicates backend logic with no test on the TS side
`src/store/mixer.ts:550-594` - testing/unit-gap - defect - effort: small
`setBusMembers`/`setBusExclude` optimistically mirror the backend's exclude-mode conversion (tested in Rust, untested in TS); drift means the UI misreports what a recording mix carries until the next fetch.
Fix: Vitest cases for the complement math, the flip preserving the effective carried set, and rejection refetch.

### TD-047: `finishOnboarding(blank)` destructively removes channels with no test
`src/store/mixer.ts:182-203` - testing/unit-gap - defect - effort: small
The blank-slate path deletes channels 2..n and renames the first, and the `replay` guard is the only thing preventing a tutorial replay from deleting the user's channels; zero coverage.
Fix: tests that replay mode makes no invokes, blank mode calls exactly the expected sequence, and non-blank mode only sets the flag.

---

## Low

### TD-048: pactl module argument/property injection via unquoted label and output name
`src-tauri/src/audio/pactl.rs:177` (and 395-401) - security/injection - defect - effort: small
Channel labels (validated only for length) are interpolated into `sink_properties=device.description={label}`; whitespace splits into extra module properties.
No shell is involved, so impact is property injection, not execution.
Fix: restrict label charset or avoid in-band delimiting for module args.

### TD-049: Spoofable `application.process.id` drives `/proc/<pid>` reads for icon resolution
`src-tauri/src/audio/icons.rs:147,182,191,208` - security/spoofing - defect - effort: medium
A malicious app can set an arbitrary PID to make Sink read another same-uid process's cgroup/environ/exe and display a spoofed identity/icon.
Fix: cross-check against `pipewire.sec.pid` or treat `/proc` results as untrusted hints.

### TD-050: Volume/mute/monitor commands skip `is_virtual_sink` validation; `MicConfig` numerics unclamped
`src-tauri/src/commands/routing.rs:61,82` - security/access-control (also rust/validation) - defect - effort: small
Unlike `route_app_to_channel`, these pass any `sink_name` to the backend, letting a compromised webview mute or max arbitrary session sinks; `MicConfig` fields documented 0-200 accept 255 and thresholds are unbounded.
Fix: validate `is_virtual_sink` (explicitly allowing `sink_mic`/bus names where intended) and clamp `MicConfig` ranges in `set_mic_config`.

### TD-051: `reset_app` wipes the config tree from a pure IPC call
`src-tauri/src/commands/settings.rs:117` - security/destructive - improvement - effort: small
Deletion is correctly scoped to Sink's own directories, so this is acceptable; if hardening is desired, gate destructive commands behind a native confirmation dialog.

### TD-052: WirePlumber conf directory created without restricted permissions
`src-tauri/src/persistence/wireplumber.rs:72` - security/permissions - improvement - effort: small
Plain `create_dir_all` + `fs::write` (default umask) versus Sink's own 0700 config dir; contents are non-sensitive routing rules, so minor.
Fix: reuse `ensure_private_dir` for consistency.

### TD-053: pactl error classification is locale-dependent
`src-tauri/src/audio/pactl.rs:78,89` - rust/subprocess - defect - effort: small
The `ServerUnreachable` detection matches English "Connection refused"; on localized systems the failure misclassifies as generic and the actionable hint is lost (`autostart.rs:96` string-matches systemctl output the same way).
Fix: `.env("LC_ALL", "C")` on the spawned commands.

### TD-054: Native request plumbing conflates disconnect with timeout, and timed-out `CreateSink` replies leak
`src-tauri/src/audio/pw_native/mod.rs:74` (also thread.rs:903) - rust/concurrency - improvement - effort: small
A loop-thread panic mid-command reports as "request timed out" instead of "backend crashed", and `pending_creates` entries for nodes that never materialize stay in the map forever.
Fix: match the recv error kind for distinct messages; expire `pending_creates` entries.

### TD-055: pactl fallback never unloads its `module-loopback`s at teardown
`src-tauri/src/audio/pactl.rs:26` - rust/lifecycle - improvement - effort: small
Cleanup relies on the server cascading the loopback's death when its source disappears; the tracked module indices exist precisely to unload on quit and should be used in `destroy_virtual_sink`.

### TD-056: Unguarded `names[names.len() - 1]` indexing in `add_channel`
`src-tauri/src/commands/channels.rs:60` - rust/panic-safety - improvement - effort: small
Non-empty by construction today, but the guarantee lives two statements away; a refactor turns this into a release-mode panic inside a Tauri command.
Fix: bind the new name explicitly or use `names.last()`.

### TD-057: `volume_percent` silently reports 100% on unparseable pactl output
`src-tauri/src/audio/pactl.rs:140` - rust/error-handling - improvement - effort: small
A parse failure masquerading as a valid reading lets the UI "correct" volumes from a fiction; at minimum log format drift.

### TD-058: `ensure_all_links` thrashes links during incremental port arrival and re-snapshots all node names per event
`src-tauri/src/audio/pw_native/thread.rs:708-771` (pairs 635-674, trigger 334-343) - performance/reconciliation - defect - effort: medium
With one input port visible, the index-wrap maps both monitors onto it; the second port's arrival destroys and recreates the links, so every device appearance/failover produces a create-destroy-create cycle per channel (pop risk), and each call rebuilds a full name-to-id HashMap and scans the whole ports map per channel-bus pair.
Idle cost is zero (event-driven); this only bites at hotplug/startup/failover.
Fix: coalesce reconciles behind a short (20-50 ms) loop timer and index ports by node id.

INVESTIGATION (2026-07-02): the loop-timer half is NOT cleanly buildable in pipewire-rs 0.8. `LoopRef::add_timer` returns a `TimerSource<'l>` bound to the loop borrow (`loop_() -> &LoopRef`), so it is not `'static`; but the registry listeners that would arm it require `Fn + 'static` (registry.rs:126), and there is no `'static` handle to a one-shot timer. The only ways to arm from the event closures are a *perpetual* repeating timer (a permanent ~20 ms wakeup - a direct idle-CPU regression, the opposite of the goal and of TD-059) or `unsafe`/leaking (against the project's no-unsafe rule).
BETTER FIX (timer-free, and actually gap-free): make the link reconcile *incremental* instead of remove-all-then-recreate - keep still-correct links, only create the added pairs and destroy the removed ones - and stop index-wrapping monitors onto a partial input set during enumeration (exact channel match plus the deliberate mono fan-out only). That removes the pop entirely with zero idle cost and is unit-testable. It is a careful refactor of the delicate link path (heal/teardown depend on `channel_links` semantics) and should land as its own focused, VM-validated change, not bundled - deferred for that reason.

### TD-059: An enabled mic chain keeps hardware capture and DSP running 24/7 with zero consumers
`src-tauri/src/audio/pw_native/mic.rs:156-176,246-258` - performance/idle-cpu - improvement - effort: large
The deliberately non-passive streams (anti-starvation) prevent the graph and capture device from ever suspending, a permanent wakeup source in the app's dominant idle state.
Fix: suspend the capture stream when `sink_mic` has had no foreign capture links for N seconds, resuming on link appearance.

### TD-060: `eq-bounce` animates `height`, forcing layout at 60 fps per active app row
`src/styles/globals.css:932-947` - performance/animation - improvement - effort: small
Fix: animate `transform: scaleY()` with `transform-origin: bottom`.

### TD-061: Icon resolver cache misses do hundreds of stat() calls while holding the global resolver lock
`src-tauri/src/audio/icons.rs:231-262` - performance/io - improvement - effort: small
Steady-state cache hits are cheap (verified), but a miss probes up to ~800 paths under the `RESOLVER` mutex, blocking concurrent stream/seen-apps commands.
Fix: probe outside the lock and re-lock to insert; read each theme's index once instead of guessing sizes.

### TD-062: PR CI is missing fmt and dependency-audit gates
`.github/workflows/dev.yml:43-47` - devops/ci-gates - improvement - effort: small
No `cargo fmt --check` and no `cargo audit`/`npm audit`; the git-pinned pipewire crate that no advisory feed watches raises the value of auditing everything else.
Fix: add fmt to the check job and a weekly scheduled audit workflow.

### TD-063: Non-atomic release publishing: 404 window and empty-release window
`.github/workflows/release.yml:120-136` - devops/release-correctness - defect - effort: medium
The dev flow deletes and recreates the release before uploading, and the Arch package lands minutes later from a separate job, so consumers see missing or partial assets during every publish.
Fix: draft-then-flip for stable; edit-and-replace instead of delete/recreate for the rolling dev release.

### TD-064: AppImage repack runs only at release time, and the archpkg script is copy-pasted between workflows
`.github/workflows/release.yml:99-111` (dev.yml:85-90, 106-128) - devops/ci-coverage - improvement - effort: medium
A Tauri bundler change breaking the repack passes PR CI and fails only on release; the duplicated 20-line archpkg script has already drifted once.
Fix: move both into `scripts/` (or a composite action) used by both workflows, and run the repack in PR CI.

### TD-065: No Dependabot/Renovate configuration
`.github/` (no dependabot.yml) - devops/automation - improvement - effort: small
Four ecosystems (npm, cargo, actions, the git-pinned pipewire rev) have no automated updates; SHA pinning (TD-020) is only sustainable with a bot.
Fix: `.github/dependabot.yml` for github-actions, npm, and cargo with grouped weekly updates.

### TD-066: No `.SRCINFO` next to the PKGBUILD
`packaging/arch/` - devops/packaging - improvement - effort: small
Costs nothing to keep a generated copy in-repo and removes a manual step from the future AUR publish.
Fix: generate in the archpkg job and fail on drift, or commit one now.

### TD-067: Floating toolchains and an untested MSRV claim
`.github/workflows/dev.yml:18-25` - devops/reproducibility - improvement - effort: small
Rust is whatever `stable` resolves to that day, Node is pinned to major only, and nothing verifies the declared `rust-version = "1.77"` still compiles.
Fix: `rust-toolchain.toml`, exact Node pin, and either an MSRV check job or dropping the claim.

### TD-068: No CI on branch pushes; direct-to-main pushes are untested
`.github/workflows/dev.yml:4` - testing/ci - improvement - effort: small
Work on feature branches gets zero automated feedback until a PR opens, and direct pushes to main bypass tests entirely (recent history shows such pushes).
Fix: add `push` triggers for the check job.

### TD-069: icons.rs tests write to a shared, fixed system temp path
`src-tauri/src/audio/icons.rs:362,379` - testing/hygiene - improvement - effort: small
A fixed `sink-test-desktop` dir shared across parallel tests and runs with no cleanup; safe today, stale-state hazard tomorrow.
Fix: `tempfile::TempDir` or unique per-test dirs.

### TD-070: ChatMix balance math lives inline in a component, untestable and untested
`src/components/MixerBoard/BalanceBar.tsx:39-49` - testing/testability - improvement - effort: small
The position derivation, clamp, center-snap, and volume-split math are pure functions trapped next to pointer handling; a sign error ducks the wrong channel.
Fix: extract `balancePos`/`balanceVolumes` into `src/lib/` and test snap, extremes, round-trip, and clamping.

### TD-071: Popover never re-positions after content changes
`src/components/Popover.tsx:31-66` - frontend/ui-robustness - defect - effort: small
Position computes once per open; ProfileMenu's expanding trigger panel and ChannelApps' poll-driven rows grow past the stale viewport clamp, making lower items unreachable.
Fix: `ResizeObserver` on the menu while open.

### TD-072: Settings screen has unhandled promise rejections and silently swallowed load errors
`src/components/Settings/SettingsScreen.tsx:96-104` - frontend/async - defect - effort: small
`get_autostart`/`get_backend_info` have no `.catch` (unhandled rejections), and `get_default_devices`/`get_prefs` swallow errors while a local error banner exists unused.
Fix: route all four through `.catch((e) => setError(String(e)))`.

### TD-073: HSlider re-registers window pointer listeners on every render
`src/components/AppList/HSlider.tsx:38` - frontend/effects - improvement - effort: small
Every 2s poll each visible row removes and re-adds two window listeners; Fader and DspSlider already solve this with a ref, HSlider is the divergent copy.
Fix: apply the same ref indirection with `[]` deps.

### TD-074: Cancelled channel drag still persists the reorder, and reorder has no keyboard path
`src/components/MixerBoard/MixerBoard.tsx:160-163`; `ChannelStrip.tsx:68-76` - frontend/ui-robustness - defect - effort: small
Escape cancels the drop but `dragend` still commits the half-done order, and the grip is a non-focusable span.
Fix: check `dropEffect === "none"` and revert; make the grip a button with arrow-key support.

### TD-075: Mic name field applies to the live node on every keystroke
`src/components/Mic/MicScreen.tsx:94` - frontend/ui-robustness - improvement - effort: small
Each typing pause fires a full `set_mic_config` apply, relabeling the virtual source through partial names while other apps' device lists flicker; MicStrip already does draft + commit-on-blur.
Fix: same draft pattern here.

### TD-076: tsconfig lacks `noUncheckedIndexedAccess`; Vite build has no explicit WebKit target
`tsconfig.json:14-17`; `vite.config.ts:7-21` - frontend/config - improvement - effort: medium
Indexed access patterns the store relies on (`s.levels[k]`, `split(" ")[0]`) type-check as non-undefined, and the build can emit syntax newer than the Linux WebKitGTK baseline.
Fix: enable the flag and fix the fallout; add `build.target: ["es2021", "safari13"]`.

### TD-077: Four hand-rolled copies of the pointer-drag slider logic
`Fader.tsx:31-44`; `HSlider.tsx:25-38`; `DspSlider.tsx:37-50`; `BalanceBar.tsx:60-73` - frontend/duplication - improvement - effort: medium
Three subtly different implementations (divergence already produced TD-073), none use pointer capture, and none are keyboard-operable.
Fix: one `usePointerDrag` hook with pointer capture; add `role="slider"` and arrow keys once, in one place.

---

## Suggested attack order

1. Ship-safety first (all small): TD-001, TD-002, TD-027, TD-026, TD-021, TD-022, TD-023 - one focused day makes the release pipeline trustworthy.
2. Data-loss and correctness (small, surgical): TD-005, TD-003, TD-004, TD-016, TD-032, TD-018.
3. User-facing routing robustness: TD-017 (the field-reported silence case) together with TD-010's plan/apply split so the fix lands tested, plus TD-058's reconcile debounce.
4. Idle footprint sweep (all small, huge daily win for a tray app): TD-008, TD-009, TD-029, TD-030, TD-031.
5. Frontend correctness/a11y cluster: TD-006, TD-007, TD-036, TD-037, TD-038, TD-039, TD-040.
6. Test debt per the 10-test plan embedded in TD-010, TD-041, TD-043, TD-044, TD-045.
7. Everything else opportunistically.

## Progress tracking

- [ ] TD-001 smoke test masks non-panic failures
- [ ] TD-002 deb/rpm missing runtime deps
- [ ] TD-003 mic node never healed
- [ ] TD-004 mutex held across blocking I/O
- [ ] TD-005 non-atomic writes + silent config reset
- [ ] TD-006 popover focus trap churn
- [ ] TD-007 mix strip state desync
- [ ] TD-008 level emitter always-on
- [ ] TD-009 poll never pauses hidden
- [ ] TD-010 failover logic untested
- [ ] TD-011 zombie PipeWire loop double audio
- [ ] TD-012 no disconnect detection
- [ ] TD-013 channels.json invariants unvalidated
- [ ] TD-014 init failures swallowed
- [ ] TD-015 volume/mute dual source of truth
- [ ] TD-016 add_channel rollback leaves sink
- [ ] TD-017 no independent default-device fallback
- [ ] TD-018 SPA-JSON escaping incomplete
- [ ] TD-019 icon-name path traversal
- [ ] TD-020 unpinned third-party actions
- [ ] TD-021 version input shell interpolation
- [ ] TD-022 stable release unvalidated
- [ ] TD-023 dev.yml no permissions block
- [ ] TD-024 no artifact checksums/signing
- [ ] TD-025 Cargo.toml never stamped
- [ ] TD-026 rust-cache dead config
- [ ] TD-027 release publishes untested code
- [ ] TD-028 pactl fork churn per tick
- [ ] TD-029 wholesale array replace re-renders
- [ ] TD-030 levels identity churn re-renders
- [ ] TD-031 rAF loops never park
- [ ] TD-032 auto_routed unbounded + ID reuse
- [ ] TD-033 seen-apps unbounded + reshipped
- [ ] TD-034 autosave read+rewrite per settle
- [ ] TD-035 pactl startup fork storm
- [ ] TD-036 error banner self-dismisses
- [ ] TD-037 mix fader invoke per pixel
- [ ] TD-038 poll overwrites optimistic drag
- [ ] TD-039 Modal/Onboarding no focus containment
- [ ] TD-040 Toggle no accessible name
- [ ] TD-041 pactl mapping untested, no seam
- [ ] TD-042 loopback text scrape untested
- [ ] TD-043 config back-compat fixtures missing
- [ ] TD-044 auto-route state machine untested
- [ ] TD-045 Rust-TS contract unenforced
- [ ] TD-046 bus complement mirror untested
- [ ] TD-047 onboarding blank path untested
- [ ] TD-048 pactl property injection via label
- [ ] TD-049 spoofable PID /proc reads
- [ ] TD-050 volume/mute unscoped + MicConfig unclamped
- [ ] TD-051 reset_app destructive IPC (accepted risk)
- [ ] TD-052 wireplumber conf dir perms
- [ ] TD-053 locale-dependent pactl errors
- [ ] TD-054 disconnect-as-timeout + pending_creates leak
- [ ] TD-055 loopbacks not unloaded at teardown
- [ ] TD-056 unguarded last-index access
- [ ] TD-057 volume parse failure reports 100%
- [ ] TD-058 link thrash on port arrival
- [ ] TD-059 mic chain 24/7 duty cycle
- [ ] TD-060 eq-bounce layout animation
- [ ] TD-061 icon resolver stats under lock
- [ ] TD-062 fmt/audit gates missing
- [ ] TD-063 non-atomic release publish windows
- [ ] TD-064 repack release-only + script duplication
- [ ] TD-065 no Dependabot
- [ ] TD-066 no .SRCINFO
- [ ] TD-067 floating toolchains / MSRV untested
- [ ] TD-068 no CI on branch pushes
- [ ] TD-069 icons.rs shared temp path
- [ ] TD-070 ChatMix math inline untested
- [ ] TD-071 popover never repositions
- [ ] TD-072 settings unhandled rejections
- [ ] TD-073 HSlider listener churn
- [ ] TD-074 drag cancel persists reorder
- [ ] TD-075 mic name applies per keystroke
- [ ] TD-076 tsconfig/vite target gaps
- [ ] TD-077 slider logic duplicated 4x
