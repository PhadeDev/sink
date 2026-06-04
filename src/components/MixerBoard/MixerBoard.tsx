import { useMixerStore } from "../../store/mixer";
import { ChannelStrip } from "./ChannelStrip";
import { MicStrip } from "./MicStrip";
import { OutputSelect } from "./OutputSelect";

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
        </div>
      </div>
    </div>
  );
}
