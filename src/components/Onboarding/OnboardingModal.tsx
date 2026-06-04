import { useState } from "react";
import { useMixerStore } from "../../store/mixer";
import { Ms } from "../Icons";

interface Step {
  icon: string;
  title: string;
  body: string;
}

const STEPS: Step[] = [
  {
    icon: "graphic_eq",
    title: "Welcome to Sink",
    body: "Sink gives every app its own place on a mixing board. Route games, voice chat, music and everything else into separate channels, then control each one independently — volume, mute, even which speakers or headphones it plays on.",
  },
  {
    icon: "tune",
    title: "Channels",
    body: "The Mixer screen is built from channels — virtual outputs with a fader, mute and live meters. Each channel can play to its own output device or follow your system default. Add, rename and re-icon them to match how you listen.",
  },
  {
    icon: "grid_view",
    title: "Apps",
    body: "Running apps show up on the Apps screen automatically. Pick a channel for each one and Sink remembers — the next time the app makes sound, it lands on the same channel. Apps you don't care about can be ignored.",
  },
  {
    icon: "podcasts",
    title: "Mixes",
    body: "Mixes are capturable sources for OBS and other recorders. The Master Mix always carries every channel; add custom mixes to record subsets — for example everything except your music.",
  },
  {
    icon: "mic",
    title: "Microphone",
    body: "The Mic screen builds a processed virtual microphone: noise gate, compressor and limiter between your hardware mic and the apps that hear you. Select it by name in Discord, OBS or anything else — and listen to yourself while you tune it.",
  },
];

/** First-run tutorial: one card per concept, then a starting-point choice. */
export function OnboardingModal() {
  const show = useMixerStore((s) => s.showOnboarding);
  const finishOnboarding = useMixerStore((s) => s.finishOnboarding);
  const [step, setStep] = useState(0);

  if (!show) return null;

  const last = step === STEPS.length; // the choice page
  const current = STEPS[step];

  return (
    <div className="modal-scrim">
      <div className="modal ob-modal" role="dialog" aria-label="Welcome to Sink">
        {last ? (
          <>
            <div className="modal-title">Pick your starting point</div>
            <p className="modal-text">
              You can add, rename and delete channels at any time — this just
              sets up the first board.
            </p>
            <div className="ob-choices">
              <button className="ob-choice" onClick={() => void finishOnboarding(false)}>
                <Ms name="dashboard" />
                <div className="ob-choice-title">Default setup</div>
                <div className="ob-choice-sub">
                  Game, Chat, Music and System channels — ready to route into
                </div>
              </button>
              <button className="ob-choice" onClick={() => void finishOnboarding(true)}>
                <Ms name="check_box_outline_blank" />
                <div className="ob-choice-title">Start blank</div>
                <div className="ob-choice-sub">
                  A single Main channel — build your own board from scratch
                </div>
              </button>
            </div>
            <div className="ob-foot">
              <button className="modal-btn" onClick={() => setStep(step - 1)}>
                Back
              </button>
              <div className="ob-dots">
                {[...STEPS, null].map((_, i) => (
                  <span key={i} className={"ob-dot" + (i === step ? " on" : "")} />
                ))}
              </div>
              <span style={{ width: 64 }} />
            </div>
          </>
        ) : (
          <>
            <div className="ob-icon">
              <Ms name={current.icon} />
            </div>
            <div className="modal-title" style={{ textAlign: "center" }}>
              {current.title}
            </div>
            <p className="modal-text ob-body">{current.body}</p>
            <div className="ob-foot">
              {step > 0 ? (
                <button className="modal-btn" onClick={() => setStep(step - 1)}>
                  Back
                </button>
              ) : (
                <button className="modal-btn" onClick={() => void finishOnboarding(false)}>
                  Skip
                </button>
              )}
              <div className="ob-dots">
                {[...STEPS, null].map((_, i) => (
                  <span key={i} className={"ob-dot" + (i === step ? " on" : "")} />
                ))}
              </div>
              <button className="modal-btn primary" onClick={() => setStep(step + 1)}>
                Next
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
