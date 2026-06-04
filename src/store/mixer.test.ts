import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// The store talks to the Rust backend through Tauri IPC; mock the boundary.
const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invoke(...args),
}));

import { useMixerStore } from "./mixer";
import type { VirtualSink } from "../types";

const channel = (name: string, volume = 100): VirtualSink => ({
  name,
  label: name.replace("sink_", ""),
  icon: null,
  volume_percent: volume,
  muted: false,
  stream_mix: true,
});

const initialState = useMixerStore.getState();

beforeEach(() => {
  vi.useFakeTimers();
  invoke.mockReset();
  invoke.mockResolvedValue(undefined);
  useMixerStore.setState(initialState, true);
});

afterEach(() => {
  vi.runOnlyPendingTimers();
  vi.useRealTimers();
});

describe("setChannelVolume", () => {
  it("updates the UI immediately and debounces the backend call", async () => {
    useMixerStore.setState({ channels: [channel("sink_game")] });
    const store = useMixerStore.getState();

    await store.setChannelVolume("sink_game", 40);
    await store.setChannelVolume("sink_game", 55);

    // Optimistic: the strip moved on the second call already…
    expect(useMixerStore.getState().channels[0].volume_percent).toBe(55);
    // …but the backend hasn't been hit yet (drag in progress).
    expect(invoke).not.toHaveBeenCalled();

    vi.advanceTimersByTime(100);
    // Only the final value of the drag reaches the backend.
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("set_channel_volume", {
      sinkName: "sink_game",
      volume: 55,
    });
  });

  it("keeps per-channel debounce keys separate", async () => {
    useMixerStore.setState({ channels: [channel("sink_game"), channel("sink_chat")] });
    const store = useMixerStore.getState();

    await store.setChannelVolume("sink_game", 10);
    await store.setChannelVolume("sink_chat", 20);
    vi.advanceTimersByTime(100);

    expect(invoke).toHaveBeenCalledTimes(2);
  });
});

describe("toggleMonitor", () => {
  it("flips optimistically and calls the backend", async () => {
    const store = useMixerStore.getState();
    await store.toggleMonitor("sink_game");

    expect(useMixerStore.getState().monitors["sink_game"]).toBe(true);
    expect(invoke).toHaveBeenCalledWith("set_monitor", {
      sinkName: "sink_game",
      enabled: true,
    });

    await useMixerStore.getState().toggleMonitor("sink_game");
    expect(useMixerStore.getState().monitors["sink_game"]).toBe(false);
  });

  it("reverts the optimistic flip when the backend rejects", async () => {
    invoke.mockRejectedValueOnce("monitoring requires the native PipeWire backend");
    const store = useMixerStore.getState();

    await store.toggleMonitor("sink_game");

    const s = useMixerStore.getState();
    expect(s.monitors["sink_game"]).toBe(false);
    expect(s.error).toContain("native PipeWire");
  });
});

describe("setLevels", () => {
  it("stores per-sink peaks", () => {
    useMixerStore.getState().setLevels({ sink_game: [0.5, 0.4] });
    expect(useMixerStore.getState().levels["sink_game"]).toEqual([0.5, 0.4]);
  });
});
