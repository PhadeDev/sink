import { useState } from "react";
import type { ReactNode } from "react";
import { useMixerStore } from "../../store/mixer";
import { MASTER_BUS } from "../../types";
import { Ms, ICON_CHOICES } from "../Icons";
import { Modal } from "../Modal";
import { ChannelStrip } from "./ChannelStrip";
import { MicStrip } from "./MicStrip";
import { BusStrip } from "./StreamMixStrip";

// UI-side gates only; the backend enforces the real limits.
const MAX_CHANNELS = 10;
const MAX_BUSES = 4;

/** Signal-flow group: header row (icon, label, count, optional +) above
 * its strips — per the updated design. */
function MixGroup({
  kind,
  icon,
  label,
  count,
  hint,
  onAdd,
  addTitle,
  children,
}: {
  kind: string;
  icon: string;
  label: string;
  count: string;
  /** Hover explanation of what this group does. */
  hint: string;
  onAdd?: () => void;
  addTitle?: string;
  children: ReactNode;
}) {
  return (
    <div className={"mix-group is-" + kind}>
      <div className="group-head" title={hint}>
        <Ms name={icon} className="gh-icon" />
        <span className="gh-label">{label}</span>
        <span className="gh-count">{count}</span>
        {onAdd && (
          <div className="gh-add-wrap">
            <button className="gh-add" onClick={onAdd} title={addTitle}>
              <Ms name="add" />
            </button>
          </div>
        )}
      </div>
      <div className="group-strips">{children}</div>
    </div>
  );
}

export function MixerBoard() {
  const channels = useMixerStore((s) => s.channels);
  const buses = useMixerStore((s) => s.buses);
  const appStreams = useMixerStore((s) => s.appStreams);
  const addChannel = useMixerStore((s) => s.addChannel);
  const addBus = useMixerStore((s) => s.addBus);
  const micConfig = useMixerStore((s) => s.micConfig);

  const [addingChannel, setAddingChannel] = useState(false);
  const [channelLabel, setChannelLabel] = useState("");
  const [channelIcon, setChannelIcon] = useState(ICON_CHOICES[0]);
  const [addingMix, setAddingMix] = useState(false);
  const [mixLabel, setMixLabel] = useState("");

  if (channels.length === 0) {
    return (
      <div className="content">
        <div className="empty-hint" style={{ margin: "auto" }}>
          Creating virtual channels…
        </div>
      </div>
    );
  }

  // Apps routed to each channel, for the strip header.
  const counts = new Map<string, number>();
  for (const stream of appStreams) {
    if (stream.assigned_sink) {
      counts.set(stream.assigned_sink, (counts.get(stream.assigned_sink) ?? 0) + 1);
    }
  }

  const closeChannelModal = () => {
    setAddingChannel(false);
    setChannelLabel("");
    setChannelIcon(ICON_CHOICES[0]);
  };
  const createChannel = () => {
    const label = channelLabel.trim();
    if (!label) return;
    void addChannel(label, channelIcon);
    closeChannelModal();
  };
  const createMix = () => {
    const label = mixLabel.trim();
    setMixLabel("");
    setAddingMix(false);
    if (label) void addBus(label);
  };

  return (
    <div className="content">
      <div className="screen-scroll" style={{ padding: 0 }}>
        <div className="mix-scroll">
          {micConfig?.enabled && (
            <>
              <MixGroup
                kind="capture"
                icon="mic"
                label="Capture"
                count="1"
                hint="Inputs: your processed microphone. Apps capture the result as Sink Mic."
              >
                <MicStrip />
              </MixGroup>
              <div className="group-div" />
            </>
          )}

          <MixGroup
            kind="playback"
            icon="apps"
            label="Channels"
            count={`${channels.length}`}
            hint="Playback: apps route into channels; each has its own volume, mute and output device."
            onAdd={channels.length < MAX_CHANNELS ? () => setAddingChannel(true) : undefined}
            addTitle="Add a channel"
          >
            {channels.map((channel) => (
              <ChannelStrip
                key={channel.name}
                channel={channel}
                appCount={counts.get(channel.name) ?? 0}
              />
            ))}
          </MixGroup>

          <div className="group-div" />

          <MixGroup
            kind="mix"
            icon="podcasts"
            label="Mixes"
            count={`${buses.length}`}
            hint="Mixes: capturable sources for OBS/recorders. Master carries everything; add mixes for subsets."
            onAdd={
              buses.filter((b) => b.name !== MASTER_BUS).length < MAX_BUSES
                ? () => setAddingMix(true)
                : undefined
            }
            addTitle="Add a mix (capturable source for OBS/recorders)"
          >
            {buses.map((bus) => (
              <BusStrip key={bus.name} bus={bus} />
            ))}
          </MixGroup>
        </div>
      </div>

      <Modal open={addingChannel} onClose={closeChannelModal} title="New channel">
        <input
          className="menu-input"
          placeholder="Channel name…"
          value={channelLabel}
          autoFocus
          maxLength={24}
          onChange={(e) => setChannelLabel(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") createChannel();
          }}
        />
        <div className="modal-label">Icon</div>
        <div className="icon-grid">
          {ICON_CHOICES.map((choice) => (
            <button
              key={choice}
              className={"icon-cell" + (choice === channelIcon ? " sel" : "")}
              onClick={() => setChannelIcon(choice)}
              aria-label={choice}
            >
              <Ms name={choice} />
            </button>
          ))}
        </div>
        <div className="modal-btns">
          <button className="modal-btn primary" onClick={createChannel} disabled={!channelLabel.trim()}>
            Create channel
          </button>
          <button className="modal-btn" onClick={closeChannelModal}>
            Cancel
          </button>
        </div>
      </Modal>

      <Modal open={addingMix} onClose={() => setAddingMix(false)} title="New mix">
        <p className="modal-text">
          A mix is a capturable source: pick which channels it carries, then
          select it by name in OBS or any recorder.
        </p>
        <input
          className="menu-input"
          placeholder="Mix name…"
          value={mixLabel}
          autoFocus
          maxLength={24}
          onChange={(e) => setMixLabel(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") createMix();
          }}
        />
        <div className="modal-btns">
          <button className="modal-btn primary" onClick={createMix} disabled={!mixLabel.trim()}>
            Create mix
          </button>
          <button className="modal-btn" onClick={() => setAddingMix(false)}>
            Cancel
          </button>
        </div>
      </Modal>
    </div>
  );
}
