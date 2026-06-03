import { useMixerStore } from "../../store/mixer";
import type { AppStream } from "../../types";
import { AppIcon } from "./AppIcon";
import { ChannelSelect } from "./ChannelSelect";
import { HSlider } from "./HSlider";

interface AppRowProps {
  stream: AppStream;
}

export function AppRow({ stream }: AppRowProps) {
  const channels = useMixerStore((s) => s.channels);
  const routeApp = useMixerStore((s) => s.routeApp);
  const setAppVolume = useMixerStore((s) => s.setAppVolume);

  const assigned = channels.find((c) => c.name === stream.assigned_sink);

  return (
    <div className="row">
      <div className="ricon">
        <AppIcon iconName={stream.icon_name} />
      </div>
      <div className="rmain">
        <div className="rtitle" title={stream.app_name}>
          {stream.app_name}
        </div>
        <div className="rsub">
          {assigned ? `→ ${assigned.label}` : "Unrouted"} · stream #{stream.index}
        </div>
      </div>
      <div className="rtrail">
        <HSlider
          value={stream.volume_percent}
          max={100}
          onChange={(v) => void setAppVolume(stream.index, v)}
        />
        <ChannelSelect
          value={stream.assigned_sink}
          onChange={(sinkName) => void routeApp(stream.index, sinkName)}
        />
      </div>
    </div>
  );
}
