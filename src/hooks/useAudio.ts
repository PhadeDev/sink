import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { useMixerStore, type Levels } from "../store/mixer";

const POLL_INTERVAL_MS = 2000;

/**
 * Boots the audio layer: creates the virtual sinks on mount, polls the app
 * stream list + device list every 2s (also the auto-route enforcement
 * trigger), subscribes to live VU level events, and auto-loads profiles
 * bound to newly connected devices (Phase 5).
 */
export function useAudio() {
  const initialize = useMixerStore((s) => s.initialize);
  const fetchAppStreams = useMixerStore((s) => s.fetchAppStreams);
  const fetchOutputs = useMixerStore((s) => s.fetchOutputs);
  const fetchSeenApps = useMixerStore((s) => s.fetchSeenApps);
  const setLevels = useMixerStore((s) => s.setLevels);
  const outputDevices = useMixerStore((s) => s.outputDevices);
  const profiles = useMixerStore((s) => s.profiles);
  const loadProfile = useMixerStore((s) => s.loadProfile);

  useEffect(() => {
    void initialize();
    const id = setInterval(() => {
      void fetchAppStreams();
      void fetchOutputs();
      void fetchSeenApps();
    }, POLL_INTERVAL_MS);
    return () => clearInterval(id);
  }, [initialize, fetchAppStreams, fetchOutputs, fetchSeenApps]);

  useEffect(() => {
    const unlisten = listen<Levels>("levels", (event) => setLevels(event.payload));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [setLevels]);

  // Hardware profile auto-switch: when a device with a bound profile
  // appears, load that profile (Sonar-style).
  const seenDevices = useRef<Set<string> | null>(null);
  useEffect(() => {
    const names = new Set(outputDevices.map((d) => d.name));
    if (seenDevices.current === null) {
      // First sample: just learn the current device set.
      if (names.size > 0) seenDevices.current = names;
      return;
    }
    for (const name of names) {
      if (!seenDevices.current.has(name)) {
        const bound = profiles.find((p) => p.trigger_device === name);
        if (bound) void loadProfile(bound.name);
      }
    }
    seenDevices.current = names;
  }, [outputDevices, profiles, loadProfile]);
}
