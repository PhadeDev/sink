import { useState } from "react";
import type { ReactNode } from "react";
import { useMixerStore } from "../../store/mixer";
import { Ms, ICON_CHOICES } from "../Icons";
import { Modal } from "../Modal";
import { ChannelStrip } from "./ChannelStrip";
import { ChatMix } from "./ChatMix";
import { MicStrip } from "./MicStrip";
import { OutputSelect } from "./OutputSelect";
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
  onAdd,
  addTitle,
  children,
}: {
  kind: string;
  icon: string;
  label: string;
  count: string;
  onAdd?: () => void;
  addTitle?: string;
  children: ReactNode;
}) {
  return (
    <div className={"mix-group is-" + kind}>
      <div className="group-head">
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
  const channelOutputs = useMixerStore((s) => s.channelOutputs);
  const setAllOutputs = useMixerStore((s) => s.setAllOutputs);
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

  // The top pill mirrors Sonar's "same device on all output channels".
  const selections = channels.map((c) => channelOutputs[c.name] ?? null);
  const allSame = selections.every((s) => s === selections[0]);

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
      <div className="mixer-top">
        <div className="mixer-out">
          <span style={{ color: "var(--fg-muted)", fontSize: "var(--fs-caption)" }}>
            OUTPUT
          </span>
          <OutputSelect
            value={allSame ? (selections[0] ?? null) : null}
            mixed={!allSame}
            onChange={(o) => void setAllOutputs(o)}
          />
        </div>
        <div style={{ flex: 1 }} />
        <ChatMix />
      </div>
      <div className="screen-scroll" style={{ padding: 0 }}>
        <div className="mix-scroll">
          {micConfig?.enabled && (
            <>
              <MixGroup kind="capture" icon="mic" label="Capture" count="1 in">
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
            onAdd={buses.length < MAX_BUSES ? () => setAddingMix(true) : undefined}
            addTitle="Add a mix (capturable source for OBS/recorders)"
          >
            {buses.map((bus) => (
              <BusStrip key={bus.name} bus={bus} />
            ))}
            {buses.length === 0 && (
              <div className="empty-hint" style={{ alignSelf: "center", padding: "var(--sp-4)" }}>
                No mixes — add one to record with OBS.
              </div>
            )}
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
