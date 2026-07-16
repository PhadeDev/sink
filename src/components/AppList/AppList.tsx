import { useState } from "react";
import { useMixerStore } from "../../store/mixer";
import type { AppStream } from "../../types";
import { Ms } from "../Icons";
import { AppRow } from "./AppRow";
import { AppIcon } from "./AppIcon";
import { InactiveRow } from "./InactiveRow";
import { relativeTime } from "../../lib/format";

/** Apps screen: live apps grouped by channel, previously-seen apps below
 * (pre-routable while closed), ignored apps tucked away at the bottom. */
export function AppList() {
  const appStreams = useMixerStore((s) => s.appStreams);
  const channels = useMixerStore((s) => s.channels);
  const seenApps = useMixerStore((s) => s.seenApps);
  const setAppIgnored = useMixerStore((s) => s.setAppIgnored);
  const forgetApp = useMixerStore((s) => s.forgetApp);
  const [showIgnored, setShowIgnored] = useState(false);

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

  const liveIdentity = new Set(appStreams.map((s) => `${s.match_prop}\0${s.match_value}`));
  const inactive = seenApps
    .filter((a) => !a.ignored && !liveIdentity.has(`${a.match_prop}\0${a.match_value}`))
    .sort((a, b) => b.last_seen - a.last_seen);
  const ignored = seenApps.filter((a) => a.ignored);

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

        {inactive.length > 0 && (
          <>
            <div className="section-label">Not running · {inactive.length}</div>
            <div className="card card-inactive">
              {inactive.map((app) => (
                <InactiveRow key={`${app.match_prop}:${app.match_value}`} app={app} />
              ))}
            </div>
          </>
        )}

        {ignored.length > 0 && (
          <>
            <button className="ignored-toggle" onClick={() => setShowIgnored((v) => !v)}>
              <Ms name={showIgnored ? "expand_less" : "expand_more"} />
              {ignored.length} ignored {ignored.length === 1 ? "app" : "apps"}
            </button>
            {showIgnored && (
              <div className="card card-inactive">
                {ignored.map((app) => (
                  <div className="row row-inactive" key={`${app.match_prop}:${app.match_value}`}>
                    <div className="ricon">
                      <AppIcon iconPath={app.icon_path} />
                    </div>
                    <div className="rmain">
                      <div className="rtitle">
                        <span className="rname">{app.alias ?? app.display_name}</span>
                      </div>
                      <div className="rsub">last seen {relativeTime(app.last_seen)}</div>
                    </div>
                    <div className="rtrail">
                      <button
                        className="rename-btn row-action"
                        title="Stop ignoring"
                        aria-label={`Stop ignoring ${app.display_name}`}
                        onClick={() => void setAppIgnored(app, false)}
                      >
                        <Ms name="visibility" style={{ fontSize: 16 }} />
                      </button>
                      <button
                        className="rename-btn row-action"
                        title="Forget - erase from history"
                        aria-label={`Forget ${app.display_name}`}
                        onClick={() => void forgetApp(app)}
                      >
                        <Ms name="delete" style={{ fontSize: 16 }} />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
