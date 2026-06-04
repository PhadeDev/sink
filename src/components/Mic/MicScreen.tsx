import { useEffect, useRef, useState } from "react";
import { useMixerStore } from "../../store/mixer";
import { MAX_MIC_GAIN, MIC_LEVEL_KEY } from "../../types";
import { perceptual } from "../../lib/audio";
import { HSlider } from "../AppList/HSlider";
import { Ms } from "../Icons";
import { Popover } from "../Popover";
import { Toggle, ToggleRow } from "../Toggle";

/** Live mono input level driven by the `levels` event stream. */
function MicLevel() {
  const fillRef = useRef<HTMLDivElement>(null);
  const level = useMixerStore((s) => s.levels[MIC_LEVEL_KEY]);
  const target = perceptual(level?.[0] ?? 0);
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
  const listening = useMixerStore((s) => s.monitors[MIC_LEVEL_KEY] ?? false);
  const toggleMonitor = useMixerStore((s) => s.toggleMonitor);
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
      ? "System default"
      : (currentDevice?.description ?? micConfig.input_device);

  return (
    <div className="content">
      <div className="screen-head">
        <h1>Microphone</h1>
        <div className="sub">Select "Sink Mic" in Discord/OBS</div>
        <div className="screen-head-actions">
          {micConfig.enabled && !micConfig.muted && (
            <span className="tag live">
              <Ms name="fiber_manual_record" style={{ fontSize: 11 }} />
              Live
            </span>
          )}
          <Toggle
            on={micConfig.enabled}
            onClick={() => void setMicConfig({ enabled: !micConfig.enabled })}
          />
        </div>
      </div>
      <div className="screen-scroll" style={{ maxWidth: 680 }}>
        {!micConfig.enabled ? (
          <div className="empty-hint">
            Mic processing is off.
            <br />
            Switch it on (top right) to create the "Sink Mic" virtual microphone.
          </div>
        ) : (
          <>
            <div className="section-label">Input</div>
            <div className="card mic-card">
              <div className="mic-device-row">
                <div className="ricon">
                  <Ms name="settings_voice" />
                </div>
                <div style={{ position: "relative", flex: 1, minWidth: 0 }}>
                  <button className="select mic-device-select" onClick={() => setDeviceOpen((o) => !o)}>
                    <span className="mic-device-name">{deviceLabel}</span>
                    <Ms name="expand_more" />
                  </button>
                  <Popover
                    open={deviceOpen}
                    onClose={() => setDeviceOpen(false)}
                    side="bottom"
                    align="start"
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
                <button
                  className={"sbtn" + (micConfig.muted ? " on-mute" : "")}
                  style={{ width: 34 }}
                  title={micConfig.muted ? "Unmute mic" : "Mute mic"}
                  onClick={() => void setMicConfig({ muted: !micConfig.muted })}
                >
                  <Ms name={micConfig.muted ? "mic_off" : "mic"} style={{ fontSize: 18 }} />
                </button>
              </div>

              <MicLevel />

              <div className="mic-gain-row">
                <span className="mic-gain-label">Gain</span>
                <HSlider
                  value={micConfig.gain_percent}
                  max={MAX_MIC_GAIN}
                  onChange={(v) => void setMicConfig({ gain_percent: v })}
                />
              </div>
            </div>

            <div className="section-label">Check your sound</div>
            <div className="card" style={{ padding: "var(--sp-2)" }}>
              <ToggleRow
                icon="headphones"
                title="Listen to yourself"
                sub="Hear the processed mic on your output — tune the gate and compressor live"
                on={listening}
                onToggle={() => void toggleMonitor(MIC_LEVEL_KEY)}
              />
            </div>

            <div className="section-label">Processing</div>
            <div className="card" style={{ padding: "var(--sp-2)" }}>
              <ToggleRow
                icon="noise_control_off"
                title="Noise gate"
                sub="Cuts the noise floor between words (-40 dB threshold)"
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
