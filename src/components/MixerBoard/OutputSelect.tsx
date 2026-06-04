import { useState } from "react";
import type { CSSProperties } from "react";
import { useMixerStore } from "../../store/mixer";
import { Ms } from "../Icons";
import { Popover } from "../Popover";

interface OutputSelectProps {
  /** Selected output node name; null = follow system default. */
  value: string | null;
  /** "Mixed" display for the all-channels pill when selections differ. */
  mixed?: boolean;
  onChange: (outputName: string | null) => void;
  /** Compact footer style (channel strip) vs pill style (mixer top bar). */
  compact?: boolean;
  popoverStyle?: CSSProperties;
}

function deviceIcon(description: string): string {
  const d = description.toLowerCase();
  if (d.includes("headphone") || d.includes("headset") || d.includes("arctis")) return "headphones";
  if (d.includes("hdmi") || d.includes("display")) return "tv";
  if (d.includes("bluetooth")) return "bluetooth";
  return "speaker";
}

export function OutputSelect({ value, mixed, onChange, compact, popoverStyle }: OutputSelectProps) {
  const [open, setOpen] = useState(false);
  const outputDevices = useMixerStore((s) => s.outputDevices);

  const current = value === null ? null : outputDevices.find((d) => d.name === value);
  const label = mixed
    ? "Per-channel"
    : value === null
      ? "System default"
      : (current?.description ?? value);
  const shortLabel = mixed ? "Mixed" : value === null ? "System out" : label.split(" ")[0];

  const items = (
    <>
      <div
        className={"menu-item" + (!mixed && value === null ? " sel" : "")}
        onClick={() => {
          onChange(null);
          setOpen(false);
        }}
      >
        <Ms name="speaker_group" />
        <span>System default</span>
        {!mixed && value === null && <Ms name="check" style={{ marginLeft: "auto" }} />}
      </div>
      {outputDevices.map((d) => (
        <div
          key={d.name}
          className={"menu-item" + (!mixed && d.name === value ? " sel" : "")}
          onClick={() => {
            onChange(d.name);
            setOpen(false);
          }}
        >
          <Ms name={deviceIcon(d.description)} />
          <span>{d.description}</span>
          {!mixed && d.name === value && <Ms name="check" style={{ marginLeft: "auto" }} />}
        </div>
      ))}
    </>
  );

  if (compact) {
    return (
      <div style={{ position: "relative" }}>
        <button className="strip-route strip-route-btn" onClick={() => setOpen((o) => !o)} title={label}>
          <Ms name="arrow_forward" />
          <span className="strip-route-name">{shortLabel}</span>
          <Ms name="expand_more" />
        </button>
        <Popover
          open={open}
          onClose={() => setOpen(false)}
          side="top"
          align="center"
          style={popoverStyle}
        >
          {items}
        </Popover>
      </div>
    );
  }

  return (
    <div style={{ position: "relative" }}>
      <button className="out-pill" onClick={() => setOpen((o) => !o)}>
        <Ms name={current ? deviceIcon(current.description) : "speaker_group"} />
        <span>{label}</span>
        <Ms name="expand_more" className="chev" />
      </button>
      <Popover open={open} onClose={() => setOpen(false)} side="bottom" align="start" style={popoverStyle}>
        {items}
      </Popover>
    </div>
  );
}
