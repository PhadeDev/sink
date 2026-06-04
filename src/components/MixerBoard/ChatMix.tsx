import { useCallback, useEffect, useRef } from "react";
import { useMixerStore } from "../../store/mixer";

/**
 * Sonar-style ChatMix: one slider balancing Game vs Chat. Center keeps
 * both at 100%; moving toward a side attenuates the other channel.
 * Stateless — the position is derived from the two channel volumes, so it
 * stays consistent with the faders and with loaded profiles.
 */
export function ChatMix() {
  const channels = useMixerStore((s) => s.channels);
  const setChannelVolume = useMixerStore((s) => s.setChannelVolume);

  const game = channels.find((c) => c.name === "sink_game");
  const chat = channels.find((c) => c.name === "sink_chat");

  const trackRef = useRef<HTMLDivElement>(null);
  const dragging = useRef(false);

  // balance ∈ [-1, 1]: -1 = full game (chat silent), +1 = full chat.
  const balance = (() => {
    if (!game || !chat) return 0;
    if (game.volume_percent >= 100 && chat.volume_percent < 100) {
      return -(1 - chat.volume_percent / 100);
    }
    if (chat.volume_percent >= 100 && game.volume_percent < 100) {
      return 1 - game.volume_percent / 100;
    }
    return 0;
  })();

  const apply = useCallback(
    (b: number) => {
      const clamped = Math.max(-1, Math.min(1, b));
      const gameVol = clamped <= 0 ? 100 : Math.round(100 * (1 - clamped));
      const chatVol = clamped >= 0 ? 100 : Math.round(100 * (1 + clamped));
      void setChannelVolume("sink_game", gameVol);
      void setChannelVolume("sink_chat", chatVol);
    },
    [setChannelVolume],
  );

  const setFromEvent = useCallback(
    (clientX: number) => {
      const el = trackRef.current;
      if (!el) return;
      const r = el.getBoundingClientRect();
      const pct = Math.max(0, Math.min(1, (clientX - r.left) / r.width));
      apply(pct * 2 - 1);
    },
    [apply],
  );

  useEffect(() => {
    const move = (e: PointerEvent) => {
      if (dragging.current) setFromEvent(e.clientX);
    };
    const up = () => {
      dragging.current = false;
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
    };
  }, [setFromEvent]);

  if (!game || !chat) return null;

  const pct = ((balance + 1) / 2) * 100;

  return (
    <div className="chatmix" title="ChatMix — balance Game vs Chat">
      <span className="chatmix-label">GAME</span>
      <div
        className="hs-track chatmix-track"
        ref={trackRef}
        onPointerDown={(e) => {
          dragging.current = true;
          setFromEvent(e.clientX);
        }}
        onDoubleClick={() => apply(0)}
      >
        <div className="chatmix-center" />
        <div className="hs-cap" style={{ left: pct + "%" }} />
      </div>
      <span className="chatmix-label">CHAT</span>
    </div>
  );
}
