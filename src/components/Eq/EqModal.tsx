import { useState } from "react";
import { useMixerStore } from "../../store/mixer";
import type { EqBand, EqConfig, VirtualSink } from "../../types";
import { defaultEqConfig, MAX_EQ_BANDS } from "../../types";
import { Modal } from "../Modal";
import { Ms } from "../Icons";
import { Toggle } from "../Toggle";
import { DspSlider } from "../Mic/DspSlider";
import { EqBandRow } from "./EqBandRow";
import { bandColor, EqCurve } from "./EqCurve";
import { EqPresetMenu } from "./EqPresetMenu";

interface EqModalProps {
  channel: VirtualSink;
  open: boolean;
  onClose: () => void;
}

const flatBand = (freq_hz: number): EqBand => ({
  kind: "peaking",
  freq_hz,
  gain_db: 0,
  q: 1,
});

const detailedLayout = (): EqBand[] => [
  { kind: "low_shelf", freq_hz: 80, gain_db: 0, q: 0.71 },
  flatBand(125),
  flatBand(160),
  flatBand(315),
  flatBand(630),
  flatBand(1250),
  flatBand(2500),
  flatBand(5000),
  flatBand(8000),
  { kind: "high_shelf", freq_hz: 10000, gain_db: 0, q: 0.71 },
];

const fineLayout = (): EqBand[] => [
  { kind: "low_shelf", freq_hz: 55, gain_db: 0, q: 0.71 },
  flatBand(90),
  flatBand(140),
  flatBand(220),
  flatBand(350),
  flatBand(550),
  flatBand(850),
  flatBand(1300),
  flatBand(2000),
  flatBand(3200),
  flatBand(5000),
  flatBand(8000),
  flatBand(10500),
  { kind: "high_shelf", freq_hz: 12000, gain_db: 0, q: 0.71 },
];

/** Per-channel parametric EQ editor: response curve, band list, preamp. */
export function EqModal({ channel, open, onClose }: EqModalProps) {
  const config = useMixerStore(
    (s) => s.eqConfigs[channel.name] ?? null,
  ) ?? defaultEqConfig();
  const setChannelEq = useMixerStore((s) => s.setChannelEq);
  const backendNative = useMixerStore((s) => s.backendNative);
  const [selected, setSelected] = useState(0);

  // Surface preset/import failures on the app's global error banner.
  const setError = (message: string) => useMixerStore.setState({ error: message });

  const apply = (next: EqConfig) => void setChannelEq(channel.name, next);

  const patchBand = (index: number, patch: Partial<EqBand>) => {
    const bands = config.bands.map((b, i) => (i === index ? { ...b, ...patch } : b));
    apply({ ...config, bands });
  };

  const addBand = () => {
    if (config.bands.length >= MAX_EQ_BANDS) return;
    const bands = [...config.bands, { kind: "peaking" as const, freq_hz: 1000, gain_db: 0, q: 1 }];
    apply({ ...config, bands });
    setSelected(bands.length - 1);
  };

  const removeBand = (index: number) => {
    if (config.bands.length <= 1) return;
    const bands = config.bands.filter((_, i) => i !== index);
    apply({ ...config, bands });
    setSelected(Math.min(selected, bands.length - 1));
  };

  const reset = () => {
    // Back to the flat starting layout; the enable switch is left alone.
    apply({ ...defaultEqConfig(), enabled: config.enabled });
    setSelected(0);
  };

  const setLayout = (bands: EqBand[]) => {
    apply({ ...config, preamp_db: 0, bands: bands.slice(0, MAX_EQ_BANDS) });
    setSelected(0);
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={`${channel.label} - Equalizer`}
      className="eqm-modal"
    >
      {backendNative === false && (
        <p className="modal-text">
          Parametric EQ requires the native PipeWire engine, which isn't
          running on this system.
        </p>
      )}
      <div className="eqm-head">
        <div className="eqm-enable">
          <Toggle
            on={config.enabled}
            onClick={() => apply({ ...config, enabled: !config.enabled })}
          />
          <span>{config.enabled ? "On" : "Off"}</span>
        </div>
        <div className="eqm-head-actions">
          <EqPresetMenu
            sinkName={channel.name}
            config={config}
            onApply={apply}
            onError={setError}
          />
          <button
            className="select eqm-iconbtn"
            onClick={reset}
            title="Reset to the flat 5-band layout"
            aria-label="Reset EQ"
          >
            <Ms name="restart_alt" style={{ fontSize: 16 }} />
          </button>
        </div>
      </div>

      <EqCurve
        config={config}
        selected={selected}
        onSelect={setSelected}
        onBandChange={patchBand}
      />
      <p className="eqm-hint">
        Drag a point to move it · scroll over it to widen or narrow ·
        double-click to flatten
      </p>

      <div className="eqm-tools" aria-label="EQ layout tools">
        <span className="eqm-count">{config.bands.length}/{MAX_EQ_BANDS} bands</span>
        <button className="select eqm-toolbtn" onClick={() => setLayout(detailedLayout())}>
          10-band flat
        </button>
        <button className="select eqm-toolbtn" onClick={() => setLayout(fineLayout())}>
          15-band fine
        </button>
      </div>

      <DspSlider
        label="Preamp"
        min={-24}
        max={24}
        step={0.5}
        unit=" dB"
        value={config.preamp_db}
        defaultValue={0}
        onChange={(v) => apply({ ...config, preamp_db: v })}
      />

      <div className="eqm-bandlist">
        <div className="eqm-bands-viewport">
          <div className="eqm-bands">
            {config.bands.map((band, i) => (
              <EqBandRow
                key={i}
                index={i}
                band={band}
                color={bandColor(i)}
                selected={i === selected}
                canRemove={config.bands.length > 1}
                onSelect={() => setSelected(i)}
                onChange={(patch) => patchBand(i, patch)}
                onRemove={() => removeBand(i)}
              />
            ))}
          </div>
        </div>
        {config.bands.length < MAX_EQ_BANDS && (
          <button className="eqm-add" onClick={addBand}>
            <Ms name="add" style={{ fontSize: 15 }} />
            Add band
          </button>
        )}
      </div>
    </Modal>
  );
}
