import { useMixerStore } from "../../store/mixer";
import type { AppStream } from "../../types";
import { Ms } from "../Icons";
import { AppRow } from "./AppRow";

/** Apps screen: per-app routing and volume, grouped by assigned channel. */
export function AppList() {
  const appStreams = useMixerStore((s) => s.appStreams);
  const channels = useMixerStore((s) => s.channels);

  const byName = (a: AppStream, b: AppStream) =>
    (a.alias ?? a.app_name).localeCompare(b.alias ?? b.app_name);

  const groups = [
    ...channels.map((c) => ({
      key: c.name,
      label: c.label,
      streams: appStreams.filter((s) => s.assigned_sink === c.name).sort(byName),
    })),
    {
      key: "unrouted",
      label: "Unrouted",
      streams: appStreams.filter((s) => !s.assigned_sink).sort(byName),
    },
  ].filter((g) => g.streams.length > 0);

  return (
    <div className="content">
      <div className="screen-head">
        <h1>Applications</h1>
        <div className="sub">Route each app's audio to a channel</div>
        <div className="screen-head-actions">
          <span className="tag">
            <Ms name="graphic_eq" />
            {appStreams.length} {appStreams.length === 1 ? "stream" : "streams"}
          </span>
        </div>
      </div>
      <div className="screen-scroll">
        {appStreams.length === 0 ? (
          <div className="empty-hint">
            No apps are playing audio.
            <br />
            Start something noisy and it will show up here.
          </div>
        ) : (
          groups.map((group) => (
            <div key={group.key}>
              <div className="section-label">
                {group.label} · {group.streams.length}
              </div>
              <div className="card">
                {group.streams.map((stream) => (
                  <AppRow key={stream.index} stream={stream} />
                ))}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
