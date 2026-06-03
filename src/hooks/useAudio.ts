import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useMixerStore, type Levels } from "../store/mixer";

const POLL_INTERVAL_MS = 2000;

/**
 * Boots the audio layer: creates the virtual sinks on mount, polls the app
 * stream list every 2s (also the auto-route enforcement trigger), and
 * subscribes to live VU level events from the native backend.
 */
export function useAudio() {
  const initialize = useMixerStore((s) => s.initialize);
  const fetchAppStreams = useMixerStore((s) => s.fetchAppStreams);
  const setLevels = useMixerStore((s) => s.setLevels);

  useEffect(() => {
    void initialize();
    const id = setInterval(() => void fetchAppStreams(), POLL_INTERVAL_MS);
    return () => clearInterval(id);
  }, [initialize, fetchAppStreams]);

  useEffect(() => {
    const unlisten = listen<Levels>("levels", (event) => setLevels(event.payload));
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [setLevels]);
}
