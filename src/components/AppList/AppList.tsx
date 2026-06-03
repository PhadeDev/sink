import { useMixerStore } from "../../store/mixer";
import { Ms } from "../Icons";
import { AppRow } from "./AppRow";

/** Apps screen: per-app routing and volume (design's Routing + Apps merged). */
export function AppList() {
  const appStreams = useMixerStore((s) => s.appStreams);

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
          <div className="card">
            {appStreams.map((stream) => (
              <AppRow key={stream.index} stream={stream} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
