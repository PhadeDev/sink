import { useMixerStore } from "../../store/mixer";
import { MAX_MIC_GAIN, MIC_LEVEL_KEY } from "../../types";
import { perceptual } from "../../lib/audio";
import { Ms } from "../Icons";
import { Fader } from "./Fader";
import { VuMeter } from "./VuMeter";

/** Mic channel strip (Phase 3): fader = chain gain, meters = processed
 * signal. Only rendered while the mic chain is enabled. */
export function MicStrip() {
  const micConfig = useMixerStore((s) => s.micConfig);
  const setMicConfig = useMixerStore((s) => s.setMicConfig);
  const level = useMixerStore((s) => s.levels[MIC_LEVEL_KEY]);

  if (!micConfig?.enabled) return null;

  const target = micConfig.muted ? 0 : perceptual(level?.[0] ?? 0);

  return (
    <>
      <div className={"strip input-strip" + (micConfig.muted ? " muted" : "")}>
      <div className="strip-head">
        <div className="strip-icon strip-icon-mic">
          <Ms name="mic" />
        </div>
        <div className="strip-name">Mic</div>
        <div className="strip-meta">capture</div>
      </div>

      <div className="strip-body">
        <Fader
          value={micConfig.gain_percent}
          max={MAX_MIC_GAIN}
          onChange={(v) => void setMicConfig({ gain_percent: v })}
        />
        <VuMeter target={target} />
      </div>

      <div className="strip-readout">
        {micConfig.gain_percent}
        <span style={{ fontSize: 11 }}>%</span>{" "}
        <span className="db">gain</span>
      </div>

      <div className="strip-btns">
        <button
          className={"sbtn" + (micConfig.muted ? " on-mute" : "")}
          onClick={() => void setMicConfig({ muted: !micConfig.muted })}
          aria-pressed={micConfig.muted}
          title={micConfig.muted ? "Unmute mic" : "Mute mic"}
        >
          <Ms name={micConfig.muted ? "mic_off" : "mic"} style={{ fontSize: 16 }} />
        </button>
      </div>

      <div className="strip-route">
        <Ms name="graphic_eq" />
        <span>Mic stream</span>
      </div>
      </div>
      {/* divider: playback channels start to the right */}
      <div className="strips-divider" aria-hidden="true" />
    </>
  );
}
