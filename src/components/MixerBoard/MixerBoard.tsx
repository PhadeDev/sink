import { useMixerStore } from "../../store/mixer";
import { ChannelStrip } from "./ChannelStrip";

export function MixerBoard() {
  const channels = useMixerStore((s) => s.channels);
  const appStreams = useMixerStore((s) => s.appStreams);

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

  return (
    <div className="content">
      <div className="screen-scroll" style={{ padding: 0 }}>
        <div className="strips">
          {channels.map((channel) => (
            <ChannelStrip
              key={channel.name}
              channel={channel}
              appCount={counts.get(channel.name) ?? 0}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
