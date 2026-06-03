interface VuMeterProps {
  active: boolean;
  /** 0–1 level (channel volume / max). */
  intensity: number;
}

/**
 * Static level indicator: shows the channel's volume ceiling, dark when
 * muted. Real-time levels arrive with the native PipeWire backend (Phase 2);
 * Phase 1 has no level data, so the meter is deliberately not animated.
 */
export function VuMeter({ active, intensity }: VuMeterProps) {
  const pct = active ? Math.max(0, Math.min(1, intensity)) * 100 : 0;
  return (
    <div className="meter">
      <div className="meter-fill" style={{ height: pct.toFixed(1) + "%" }} />
    </div>
  );
}
