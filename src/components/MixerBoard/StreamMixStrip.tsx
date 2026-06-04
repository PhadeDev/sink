import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMixerStore } from "../../store/mixer";
import type { BusDef } from "../../types";
import { MAX_VOLUME } from "../../types";
import { perceptual, volToDb } from "../../lib/audio";
import { Ms } from "../Icons";
import { Modal } from "../Modal";
import { Popover } from "../Popover";
import { Fader } from "./Fader";
import { VuMeter } from "./VuMeter";

const MAX_BUSES = 4;

/**
 * A mix (record bus): aggregates the chosen channels into a capturable
 * source. The label is exactly the device name recorders display — rename
 * it and OBS sees the new name. Volume/mute shape what recorders hear,
 * not what you hear.
 */
function BusStrip({ bus }: { bus: BusDef }) {
  const channels = useMixerStore((s) => s.channels);
  const setBusMembers = useMixerStore((s) => s.setBusMembers);
  const renameBus = useMixerStore((s) => s.renameBus);
  const removeBus = useMixerStore((s) => s.removeBus);
  const level = useMixerStore((s) => s.levels[bus.name]);

  const [volume, setVolume] = useState(100);
  const [muted, setMuted] = useState(false);
  const [managing, setManaging] = useState(false);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [confirmingDelete, setConfirmingDelete] = useState(false);

  const amplitude = Math.max(level?.[0] ?? 0, level?.[1] ?? 0);

  const applyVolume = (v: number) => {
    setVolume(v);
    void invoke("set_channel_volume", { sinkName: bus.name, volume: v }).catch(() => {});
  };
  const toggleMute = () => {
    const next = !muted;
    setMuted(next);
    void invoke("toggle_channel_mute", { sinkName: bus.name, muted: next }).catch(() => {});
  };
  const commitRename = () => {
    setEditing(false);
    const label = draft.trim();
    if (label && label !== bus.label) void renameBus(bus.name, label);
  };
  const toggleMember = (channelName: string) => {
    const next = bus.channels.includes(channelName)
      ? bus.channels.filter((c) => c !== channelName)
      : [...bus.channels, channelName];
    void setBusMembers(bus.name, next);
  };

  return (
    <div className={"strip bus-strip" + (muted ? " muted" : "")}>
      <button
        className="strip-delete"
        aria-label={`Delete mix ${bus.label}`}
        title="Delete mix"
        onClick={() => setConfirmingDelete(true)}
      >
        <Ms name="close" style={{ fontSize: 13 }} />
      </button>

      <div className="strip-head">
        <div className="strip-icon strip-icon-bus">
          <Ms name="radio_button_checked" />
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
            title='Double-click to rename — recorders see this name'
            onDoubleClick={() => {
              setDraft(bus.label);
              setEditing(true);
            }}
          >
            {bus.label}
          </div>
        )}
        <div style={{ position: "relative" }}>
          <button
            className="strip-meta strip-meta-btn"
            title="Choose which channels this mix carries"
            onClick={() => setManaging(true)}
          >
            {bus.channels.length} {bus.channels.length === 1 ? "channel" : "channels"}
            <Ms name="expand_more" style={{ fontSize: 13 }} />
          </button>
          <Popover
            open={managing}
            onClose={() => setManaging(false)}
            side="bottom"
            align="center"
            style={{ minWidth: 220 }}
          >
            {channels.map((c) => {
              const checked = bus.channels.includes(c.name);
              return (
                <div key={c.name} className="menu-item" onClick={() => toggleMember(c.name)}>
                  <Ms
                    name={checked ? "check_box" : "check_box_outline_blank"}
                    style={checked ? { color: "var(--accent-hover)" } : undefined}
                  />
                  <span>{c.label}</span>
                </div>
              );
            })}
          </Popover>
        </div>
      </div>

      <div className="strip-body">
        <Fader value={volume} max={MAX_VOLUME} onChange={applyVolume} />
        <VuMeter target={muted ? 0 : perceptual(amplitude)} />
      </div>

      <div className="strip-readout">
        {volume}
        <span style={{ fontSize: 11 }}>%</span> <span className="db">{volToDb(volume)}</span>
      </div>

      <div className="strip-btns">
        <button
          className={"sbtn" + (muted ? " on-mute" : "")}
          onClick={toggleMute}
          aria-pressed={muted}
          title={muted ? "Unmute this mix" : "Mute this mix (recorders hear silence)"}
        >
          <Ms name={muted ? "volume_off" : "volume_up"} style={{ fontSize: 16 }} />
        </button>
      </div>

      <div
        className="strip-route"
        title={`Select "${bus.label}" as an audio source in OBS or any recorder`}
      >
        <Ms name="radio_button_checked" />
        <span className="strip-route-name">capturable</span>
      </div>

      <Modal
        open={confirmingDelete}
        onClose={() => setConfirmingDelete(false)}
        title={`Delete mix "${bus.label}"?`}
      >
        <p className="modal-text">
          Recorders capturing "{bus.label}" will go silent. Channels are unaffected.
        </p>
        <div className="modal-btns">
          <button
            className="modal-btn danger"
            onClick={() => {
              setConfirmingDelete(false);
              void removeBus(bus.name);
            }}
          >
            Delete mix
          </button>
          <button className="modal-btn" onClick={() => setConfirmingDelete(false)}>
            Cancel
          </button>
        </div>
      </Modal>
    </div>
  );
}

/** The mixes section: bus strips behind their divider + an add affordance. */
export function BusStrips() {
  const buses = useMixerStore((s) => s.buses);
  const addBus = useMixerStore((s) => s.addBus);
  const [adding, setAdding] = useState(false);
  const [label, setLabel] = useState("");

  const create = () => {
    const trimmed = label.trim();
    setLabel("");
    setAdding(false);
    if (trimmed) void addBus(trimmed);
  };

  return (
    <>
      <div className="strips-divider" aria-hidden="true" />
      {buses.map((bus) => (
        <BusStrip key={bus.name} bus={bus} />
      ))}
      {buses.length < MAX_BUSES && (
        <button
          className="strip strip-add strip-add-bus"
          onClick={() => setAdding(true)}
          title="Add a mix — a capturable source for OBS/recorders"
        >
          <Ms name="add" style={{ fontSize: 22 }} />
          <span className="strip-add-label">Add mix</span>
        </button>
      )}

      <Modal open={adding} onClose={() => setAdding(false)} title="New mix">
        <p className="modal-text">
          A mix is a capturable source: pick which channels it carries, then
          select it by name in OBS or any recorder.
        </p>
        <input
          className="menu-input"
          placeholder="Mix name…"
          value={label}
          autoFocus
          maxLength={24}
          onChange={(e) => setLabel(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") create();
          }}
        />
        <div className="modal-btns">
          <button className="modal-btn primary" onClick={create} disabled={!label.trim()}>
            Create mix
          </button>
          <button className="modal-btn" onClick={() => setAdding(false)}>
            Cancel
          </button>
        </div>
      </Modal>
    </>
  );
}
