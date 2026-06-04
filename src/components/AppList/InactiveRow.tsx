import { useMixerStore } from "../../store/mixer";
import type { SeenApp } from "../../types";
import { Ms } from "../Icons";
import { AppIcon } from "./AppIcon";
import { ChannelSelect } from "./ChannelSelect";

export function relativeTime(unixSeconds: number): string {
  const delta = Math.max(0, Math.floor(Date.now() / 1000) - unixSeconds);
  if (delta < 90) return "just now";
  if (delta < 3600) return `${Math.round(delta / 60)}m ago`;
  if (delta < 86400) return `${Math.round(delta / 3600)}h ago`;
  return `${Math.round(delta / 86400)}d ago`;
}

/**
 * A previously-seen app that isn't currently playing. Routing edits here
 * are "pre-routing": they take effect the moment the app next plays audio.
 */
export function InactiveRow({ app }: { app: SeenApp }) {
  const setAppAssignment = useMixerStore((s) => s.setAppAssignment);
  const setAppIgnored = useMixerStore((s) => s.setAppIgnored);
  const forgetApp = useMixerStore((s) => s.forgetApp);

  return (
    <div className="row row-inactive">
      <div className="ricon">
        <AppIcon iconName={app.icon_name} appName={app.display_name} />
      </div>
      <div className="rmain">
        <div className="rtitle" title={app.match_value}>
          <span className="rname">{app.alias ?? app.display_name}</span>
        </div>
        <div className="rsub">last seen {relativeTime(app.last_seen)}</div>
      </div>
      <div className="rtrail">
        <button
          className="rename-btn row-action"
          title="Ignore — hide this app from Sink"
          aria-label={`Ignore ${app.display_name}`}
          onClick={() => void setAppIgnored(app, true)}
        >
          <Ms name="visibility_off" style={{ fontSize: 16 }} />
        </button>
        <button
          className="rename-btn row-action"
          title="Forget — erase from history (and its routing/alias)"
          aria-label={`Forget ${app.display_name}`}
          onClick={() => void forgetApp(app)}
        >
          <Ms name="delete" style={{ fontSize: 16 }} />
        </button>
        <ChannelSelect
          value={app.assigned_sink}
          onChange={(sinkName) => void setAppAssignment(app, sinkName === "" ? null : sinkName)}
        />
      </div>
    </div>
  );
}
