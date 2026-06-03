import { useEffect } from "react";
import { useMixerStore } from "../store/mixer";

const POLL_INTERVAL_MS = 2000;

/**
 * Boots the audio layer: creates the virtual sinks on mount and polls the
 * app stream list every 2s to pick up newly started / closed apps.
 * (Phase 2 will replace polling with native PipeWire events.)
 */
export function useAudio() {
  const initialize = useMixerStore((s) => s.initialize);
  const fetchAppStreams = useMixerStore((s) => s.fetchAppStreams);

  useEffect(() => {
    void initialize();
    const id = setInterval(() => void fetchAppStreams(), POLL_INTERVAL_MS);
    return () => clearInterval(id);
  }, [initialize, fetchAppStreams]);
}
