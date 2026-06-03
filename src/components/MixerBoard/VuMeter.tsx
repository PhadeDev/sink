import { useEffect, useRef } from "react";

interface VuMeterProps {
  /** Peak amplitude target 0–1 (real level from the native backend). */
  target: number;
}

/**
 * Live level meter with peak-hold. Targets arrive at 10 Hz from the
 * backend's `levels` events; an rAF loop smooths toward them (fast attack,
 * slow release) outside React state. Under the pactl fallback backend no
 * events arrive and the meter rests at zero.
 */
export function VuMeter({ target }: VuMeterProps) {
  const fillRef = useRef<HTMLDivElement>(null);
  const peakRef = useRef<HTMLDivElement>(null);
  const targetRef = useRef(0);
  targetRef.current = target;

  useEffect(() => {
    let raf = 0;
    let smooth = 0;
    let peak = 0;
    const tick = () => {
      const t = targetRef.current;
      smooth += (t - smooth) * (t > smooth ? 0.5 : 0.12);
      peak = Math.max(peak * 0.985, smooth);
      if (fillRef.current) fillRef.current.style.height = (smooth * 100).toFixed(1) + "%";
      if (peakRef.current) peakRef.current.style.bottom = (peak * 100).toFixed(1) + "%";
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);

  return (
    <div className="meter">
      <div className="meter-fill" ref={fillRef} style={{ height: "0%" }} />
      <div className="meter-peak" ref={peakRef} style={{ bottom: "0%" }} />
    </div>
  );
}
