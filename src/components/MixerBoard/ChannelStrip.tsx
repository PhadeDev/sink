import { useMixerStore } from "../../store/mixer";
import type { VirtualSink } from "../../types";
import { MAX_VOLUME } from "../../types";
import { channelIcon, Ms } from "../Icons";
import { Fader } from "./Fader";
import { VuMeter } from "./VuMeter";

function volToDb(v: number): string {
  if (v === 0) return "-∞";
  const db = 20 * Math.log10(v / 100);
  return (db >= 0 ? "+" : "") + db.toFixed(1) + " dB";
}

interface ChannelStripProps {
  channel: VirtualSink;
  appCount: number;
}

export function ChannelStrip({ channel, appCount }: ChannelStripProps) {
  const setChannelVolume = useMixerStore((s) => s.setChannelVolume);
  const toggleMute = useMixerStore((s) => s.toggleMute);

  const intensity = channel.volume_percent / MAX_VOLUME;

  return (
    <div className={"strip" + (channel.muted ? " muted" : "")}>
      <div className="strip-head">
        <div className="strip-icon">
          <Ms name={channelIcon(channel.name)} />
        </div>
        <div className="strip-name">{channel.label}</div>
        <div className="strip-meta">
          {appCount} {appCount === 1 ? "app" : "apps"}
        </div>
      </div>

      <div className="strip-body">
        <VuMeter active={!channel.muted} intensity={intensity} />
        <Fader
          value={channel.volume_percent}
          max={MAX_VOLUME}
          onChange={(v) => void setChannelVolume(channel.name, v)}
        />
        <VuMeter active={!channel.muted} intensity={intensity} />
      </div>

      <div className="strip-readout">
        {channel.volume_percent}
        <span style={{ fontSize: 11 }}>%</span>{" "}
        <span className="db">{volToDb(channel.volume_percent)}</span>
      </div>

      <div className="strip-btns">
        <button
          className={"sbtn" + (channel.muted ? " on-mute" : "")}
          onClick={() => void toggleMute(channel.name, !channel.muted)}
          aria-pressed={channel.muted}
          title="Mute"
        >
          M
        </button>
      </div>

      <div className="strip-route">
        <Ms name="arrow_forward" />
        <span>System out</span>
      </div>
    </div>
  );
}
