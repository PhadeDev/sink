import { useEffect, useRef, useState } from "react";
import { useMixerStore } from "../../store/mixer";
import { MAX_MIC_GAIN, MIC_LEVEL_KEY } from "../../types";
import { HSlider } from "../AppList/HSlider";
import { Ms } from "../Icons";
import { Popover } from "../Popover";

function Toggle({ on, onClick }: { on: boolean; onClick: () => void }) {
  return <button className={"toggle" + (on ? " on" : "")} onClick={onClick} aria-pressed={on} />;
}

function ToggleRow({
  icon,
  title,
  sub,
  on,
  onToggle,
}: {
  icon: string;
  title: string;
  sub: string;
  on: boolean;
  onToggle: () => void;
}) {
  return (
    <div className="row">
      <div className="ricon">
        <Ms name={icon} />
      </div>
      <div className="rmain">
        <div className="rtitle">{title}</div>
        <div className="rsub">{sub}</div>
      </div>
      <Toggle on={on} onClick={onToggle} />
    </div>
  );
}

/** Live mono input level driven by the `levels` event stream. */
function MicLevel() {
  const fillRef = useRef<HTMLDivElement>(null);
  const level = useMixerStore((s) => s.levels[MIC_LEVEL_KEY]);
  const target = Math.min(1, Math.sqrt(Math.max(0, level?.[0] ?? 0)));
  const targetRef = useRef(0);
  targetRef.current = target;

  useEffect(() => {
    let raf = 0;
    let smooth = 0;
    const tick = () => {
      const t = targetRef.current;
      smooth += (t - smooth) * (t > smooth ? 0.5 : 0.12);
      if (fillRef.current) fillRef.current.style.width = (smooth * 100).toFixed(1) + "%";
      raf = requestAnimationFrame(tick);
    };
    raf = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(raf);
  }, []);

  return (
    <div className="mini-meter">
      <div className="mf" ref={fillRef} style={{ width: "0%" }} />
    </div>
  );
}

export function MicScreen() {
  const micConfig = useMixerStore((s) => s.micConfig);
  const inputDevices = useMixerStore((s) => s.inputDevices);
  const setMicConfig = useMixerStore((s) => s.setMicConfig);
  const [deviceOpen, setDeviceOpen] = useState(false);

  if (!micConfig) {
    return (
      <div className="content">
        <div className="empty-hint" style={{ margin: "auto" }}>
          Loading mic configuration…
        </div>
      </div>
    );
  }

  const currentDevice = inputDevices.find((d) => d.name === micConfig.input_device);
  const deviceLabel =
    micConfig.input_device === null
      ? "System default mic"
      : (currentDevice?.description ?? micConfig.input_device);

  return (
    <div className="content">
      <div className="screen-head">
        <h1>Microphone</h1>
        <div className="sub">Processed virtual mic — select "Sink Mic" in Discord/OBS</div>
        <div className="screen-head-actions">
          {micConfig.enabled && !micConfig.muted && (
            <span className="tag live">
              <Ms name="fiber_manual_record" style={{ fontSize: 11 }} />
              Live
            </span>
          )}
        </div>
      </div>
      <div className="screen-scroll" style={{ maxWidth: 720 }}>
        <div className="card" style={{ padding: "var(--sp-2)" }}>
          <ToggleRow
            icon="mic"
            title="Mic processing"
            sub='Creates the "Sink Mic" virtual microphone'
            on={micConfig.enabled}
            onToggle={() => void setMicConfig({ enabled: !micConfig.enabled })}
          />
        </div>

        {micConfig.enabled && (
          <>
            <div className="section-label">Input device</div>
            <div className="card" style={{ padding: "var(--sp-2)" }}>
              <div className="row">
                <div className="ricon">
                  <Ms name="settings_voice" />
                </div>
                <div className="rmain">
                  <div className="rtitle">{deviceLabel}</div>
                  <div className="rsub">Capture source for the chain</div>
                </div>
                <div style={{ position: "relative" }}>
                  <button className="select" onClick={() => setDeviceOpen((o) => !o)}>
                    <span>Change</span>
                    <Ms name="expand_more" />
                  </button>
                  <Popover
                    open={deviceOpen}
                    onClose={() => setDeviceOpen(false)}
                    style={{ top: 38, right: 0 }}
                  >
                    <div
                      className={"menu-item" + (micConfig.input_device === null ? " sel" : "")}
                      onClick={() => {
                        void setMicConfig({ input_device: null });
                        setDeviceOpen(false);
                      }}
                    >
                      <Ms name="mic" />
                      <span>System default</span>
                    </div>
                    {inputDevices.map((d) => (
                      <div
                        key={d.name}
                        className={
                          "menu-item" + (d.name === micConfig.input_device ? " sel" : "")
                        }
                        onClick={() => {
                          void setMicConfig({ input_device: d.name });
                          setDeviceOpen(false);
                        }}
                      >
                        <Ms name="mic" />
                        <span>{d.description}</span>
                      </div>
                    ))}
                  </Popover>
                </div>
              </div>
            </div>

            <div className="section-label">Input level</div>
            <div className="card">
              <MicLevel />
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "var(--sp-4)",
                  marginTop: "var(--sp-4)",
                }}
              >
                <span style={{ fontSize: "var(--fs-meta)", color: "var(--fg-secondary)", width: 64 }}>
                  Gain
                </span>
                <HSlider
                  value={micConfig.gain_percent}
                  max={MAX_MIC_GAIN}
                  onChange={(v) => void setMicConfig({ gain_percent: v })}
                />
                <button
                  className={"sbtn" + (micConfig.muted ? " on-mute" : "")}
                  style={{ width: 34 }}
                  title="Mute mic"
                  onClick={() => void setMicConfig({ muted: !micConfig.muted })}
                >
                  <Ms name={micConfig.muted ? "mic_off" : "mic"} style={{ fontSize: 18 }} />
                </button>
              </div>
            </div>

            <div className="section-label">Processing — gate → gain → compressor → limiter</div>
            <div className="card" style={{ padding: "var(--sp-2)" }}>
              <ToggleRow
                icon="noise_control_off"
                title="Noise gate"
                sub="Cuts the noise floor between words (-45 dB threshold)"
                on={micConfig.gate_enabled}
                onToggle={() => void setMicConfig({ gate_enabled: !micConfig.gate_enabled })}
              />
              <ToggleRow
                icon="compress"
                title="Compressor"
                sub="Evens out loud peaks and quiet speech (3:1 ratio)"
                on={micConfig.comp_enabled}
                onToggle={() => void setMicConfig({ comp_enabled: !micConfig.comp_enabled })}
              />
              <ToggleRow
                icon="vertical_align_center"
                title="Limiter"
                sub="Hard ceiling at -1 dBFS — no clipping downstream"
                on={micConfig.limiter_enabled}
                onToggle={() =>
                  void setMicConfig({ limiter_enabled: !micConfig.limiter_enabled })
                }
              />
            </div>
          </>
        )}
      </div>
    </div>
  );
}
