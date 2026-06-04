import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useMixerStore } from "../../store/mixer";
import { MAX_VOLUME } from "../../types";
import { perceptual, volToDb } from "../../lib/audio";
import { Ms } from "../Icons";
import { Popover } from "../Popover";
import { Fader } from "./Fader";
import { VuMeter } from "./VuMeter";

const STREAM_MIX = "sink_stream";

/**
 * The record bus, visible: aggregates the channels marked "include" and
 * feeds recorders (select "Sink Stream Mix" in OBS). It carries channels,
 * not apps — which is why it can't be deleted or have apps routed into it
 * directly. Volume/mute shape what recorders hear, not what you hear.
 */
export function StreamMixStrip() {
  const channels = useMixerStore((s) => s.channels);
  const setChannelStreamMix = useMixerStore((s) => s.setChannelStreamMix);
  const level = useMixerStore((s) => s.levels[STREAM_MIX]);

  // The bus resets to 100%/unmuted with the session (like channels at
  // init); track its state locally and push to the node.
  const [volume, setVolume] = useState(100);
  const [muted, setMuted] = useState(false);
  const [managing, setManaging] = useState(false);

  const included = channels.filter((c) => c.stream_mix);
  const amplitude = Math.max(level?.[0] ?? 0, level?.[1] ?? 0);

  const applyVolume = (v: number) => {
    setVolume(v);
    void invoke("set_channel_volume", { sinkName: STREAM_MIX, volume: v }).catch(() => {});
  };
  const toggleMute = () => {
    const next = !muted;
    setMuted(next);
    void invoke("toggle_channel_mute", { sinkName: STREAM_MIX, muted: next }).catch(() => {});
  };

  return (
    <>
      <div className="strips-divider" aria-hidden="true" />
      <div className={"strip bus-strip" + (muted ? " muted" : "")}>
        <div className="strip-head">
          <div className="strip-icon strip-icon-bus">
            <Ms name="radio_button_checked" />
          </div>
          <div className="strip-name">Stream Mix</div>
          <div style={{ position: "relative" }}>
            <button
              className="strip-meta strip-meta-btn"
              title="Choose which channels recorders hear"
              onClick={() => setManaging(true)}
            >
              {included.length} {included.length === 1 ? "channel" : "channels"}
              <Ms name="expand_more" style={{ fontSize: 13 }} />
            </button>
            <Popover
              open={managing}
              onClose={() => setManaging(false)}
              side="bottom"
              align="center"
              style={{ minWidth: 220 }}
            >
              {channels.map((c) => (
                <div
                  key={c.name}
                  className="menu-item"
                  onClick={() => void setChannelStreamMix(c.name, !c.stream_mix)}
                >
                  <Ms
                    name={c.stream_mix ? "check_box" : "check_box_outline_blank"}
                    style={c.stream_mix ? { color: "var(--accent-hover)" } : undefined}
                  />
                  <span>{c.label}</span>
                </div>
              ))}
            </Popover>
          </div>
        </div>

        <div className="strip-body">
          <Fader value={volume} max={MAX_VOLUME} onChange={applyVolume} />
          <VuMeter target={muted ? 0 : perceptual(amplitude)} />
        </div>

        <div className="strip-readout">
          {volume}
          <span style={{ fontSize: 11 }}>%</span> <span className="db">{volToDb(volume)}</span>
        </div>

        <div className="strip-btns">
          <button
            className={"sbtn" + (muted ? " on-mute" : "")}
            onClick={toggleMute}
            aria-pressed={muted}
            title={muted ? "Unmute the record bus" : "Mute the record bus"}
          >
            <Ms name={muted ? "volume_off" : "volume_up"} style={{ fontSize: 16 }} />
          </button>
        </div>

        <div className="strip-route">
          <Ms name="radio_button_checked" />
          <span>OBS capture</span>
        </div>
      </div>
    </>
  );
}
