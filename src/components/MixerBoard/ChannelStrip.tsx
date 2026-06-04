import { useState } from "react";
import { useMixerStore } from "../../store/mixer";
import type { VirtualSink } from "../../types";
import { MAX_VOLUME } from "../../types";
import { channelIcon, Ms } from "../Icons";
import { Popover } from "../Popover";
import { Fader } from "./Fader";
import { OutputSelect } from "./OutputSelect";
import { VuMeter } from "./VuMeter";

function volToDb(v: number): string {
  if (v === 0) return "-∞";
  const db = 20 * Math.log10(v / 100);
  return (db >= 0 ? "+" : "") + db.toFixed(1) + " dB";
}

interface ChannelStripProps {
  channel: VirtualSink;
  appCount: number;
}

/** Map a linear peak amplitude to a perceptual meter height. */
function perceptual(amplitude: number): number {
  return Math.min(1, Math.sqrt(Math.max(0, amplitude)));
}

export function ChannelStrip({ channel, appCount }: ChannelStripProps) {
  const setChannelVolume = useMixerStore((s) => s.setChannelVolume);
  const toggleMute = useMixerStore((s) => s.toggleMute);
  const level = useMixerStore((s) => s.levels[channel.name]);
  const output = useMixerStore((s) => s.channelOutputs[channel.name] ?? null);
  const setChannelOutput = useMixerStore((s) => s.setChannelOutput);
  const renameChannel = useMixerStore((s) => s.renameChannel);
  const removeChannel = useMixerStore((s) => s.removeChannel);
  const channelCount = useMixerStore((s) => s.channels.length);

  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  const commitRename = () => {
    setEditing(false);
    const label = draft.trim();
    if (label && label !== channel.label) {
      void renameChannel(channel.name, label);
    }
  };

  // Mono meter: show the louder of L/R (stereo split wasn't earning its
  // width — real per-side metering returns if anyone asks).
  const amplitude = Math.max(level?.[0] ?? 0, level?.[1] ?? 0);

  return (
    <div className={"strip" + (channel.muted ? " muted" : "")}>
      {channelCount > 1 && (
        <div className="strip-delete-anchor">
          <button
            className="strip-delete"
            aria-label={`Delete channel ${channel.label}`}
            title="Delete channel"
            onClick={() => setConfirmingDelete(true)}
          >
            <Ms name="close" style={{ fontSize: 13 }} />
          </button>
          <Popover
            open={confirmingDelete}
            onClose={() => setConfirmingDelete(false)}
            style={{ top: 24, right: 0, minWidth: 230 }}
          >
            <div className="menu-hint">
              Apps on this channel return to the default output.
            </div>
            <div
              className="menu-item menu-item-danger"
              onClick={() => {
                setConfirmingDelete(false);
                void removeChannel(channel.name);
              }}
            >
              <Ms name="delete" />
              <span>Delete "{channel.label}"</span>
            </div>
            <div className="menu-item" onClick={() => setConfirmingDelete(false)}>
              <Ms name="close" />
              <span>Cancel</span>
            </div>
          </Popover>
        </div>
      )}
      <div className="strip-head">
        <div className="strip-icon">
          <Ms name={channelIcon(channel.name)} />
        </div>
        {editing ? (
          <input
            className="menu-input strip-name-input"
            value={draft}
            autoFocus
            maxLength={24}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commitRename}
            onKeyDown={(e) => {
              if (e.key === "Enter") commitRename();
              if (e.key === "Escape") setEditing(false);
            }}
          />
        ) : (
          <div
            className="strip-name strip-name-editable"
            title="Double-click to rename"
            onDoubleClick={() => {
              setDraft(channel.label);
              setEditing(true);
            }}
          >
            {channel.label}
          </div>
        )}
        <div className="strip-meta">
          {appCount} {appCount === 1 ? "app" : "apps"}
        </div>
      </div>

      <div className="strip-body">
        <Fader
          value={channel.volume_percent}
          max={MAX_VOLUME}
          onChange={(v) => void setChannelVolume(channel.name, v)}
        />
        <VuMeter target={channel.muted ? 0 : perceptual(amplitude)} />
      </div>

      <div className="strip-readout">
        {channel.volume_percent}
        <span style={{ fontSize: 11 }}>%</span>{" "}
        <span className="db">{volToDb(channel.volume_percent)}</span>
      </div>

      <div className="strip-btns">
        <button
          className={"sbtn" + (channel.muted ? " on-mute" : "")}
          onClick={() => void toggleMute(channel.name, !channel.muted)}
          aria-pressed={channel.muted}
          title="Mute"
        >
          M
        </button>
      </div>

      <OutputSelect
        compact
        value={output}
        onChange={(o) => void setChannelOutput(channel.name, o)}
      />
    </div>
  );
}
