import { useState } from "react";
import { useMixerStore } from "../../store/mixer";
import { Ms } from "../Icons";
import { ChannelStrip } from "./ChannelStrip";
import { ChatMix } from "./ChatMix";
import { MicStrip } from "./MicStrip";
import { OutputSelect } from "./OutputSelect";

const MAX_CHANNELS = 10;

/** Ghost strip that turns into an inline input to create a channel. */
function AddChannelStrip() {
  const addChannel = useMixerStore((s) => s.addChannel);
  const [editing, setEditing] = useState(false);
  const [label, setLabel] = useState("");

  const commit = () => {
    setEditing(false);
    const trimmed = label.trim();
    setLabel("");
    if (trimmed) void addChannel(trimmed);
  };

  if (editing) {
    return (
      <div className="strip strip-add">
        <input
          className="menu-input strip-name-input"
          placeholder="Channel name…"
          value={label}
          autoFocus
          maxLength={24}
          onChange={(e) => setLabel(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter") commit();
            if (e.key === "Escape") setEditing(false);
          }}
        />
      </div>
    );
  }

  return (
    <button className="strip strip-add" onClick={() => setEditing(true)} title="Add a channel">
      <Ms name="add" style={{ fontSize: 22 }} />
      <span className="strip-add-label">Add channel</span>
    </button>
  );
}

export function MixerBoard() {
  const channels = useMixerStore((s) => s.channels);
  const appStreams = useMixerStore((s) => s.appStreams);
  const channelOutputs = useMixerStore((s) => s.channelOutputs);
  const setAllOutputs = useMixerStore((s) => s.setAllOutputs);

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

  // The top pill mirrors Sonar's "same device on all output channels":
  // shows the common choice, or "Mixed" when strips diverge.
  const selections = channels.map((c) => channelOutputs[c.name] ?? null);
  const allSame = selections.every((s) => s === selections[0]);

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
        <div className="strips">
          {channels.map((channel) => (
            <ChannelStrip
              key={channel.name}
              channel={channel}
              appCount={counts.get(channel.name) ?? 0}
            />
          ))}
          <MicStrip />
          {channels.length < MAX_CHANNELS && <AddChannelStrip />}
        </div>
      </div>
    </div>
  );
}
