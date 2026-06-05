import { useEffect, useState } from "react";
import { useMixerStore } from "../../store/mixer";
import { Ms } from "../Icons";

interface Step {
  icon: string;
  title: string;
  body: string;
  /** Monospace signal-flow sketch shown under the body. */
  diagram?: string;
}

// Three short cards: the mental model first (as a picture), then the two
// things you can't discover just by looking at the screen.
const STEPS: Step[] = [
  {
    icon: "graphic_eq",
    title: "Your sound, on a board",
    body: "Every app's audio lands on a channel you control. Send each channel to your ears — and tap any group as a recording.",
    diagram:
      " apps ─► channels ─► your ears\n              └────► a Mix ─► OBS",
  },
  {
    icon: "grid_view",
    title: "Sort your apps",
    body: "New apps appear on their own. Drop each onto a channel — game, chat, music — and Sink keeps it there next time.",
  },
  {
    icon: "mic",
    title: "A better mic",
    body: "Sink builds a cleaned-up mic — gated, compressed, leveled. Pick it in Discord or OBS, and hear yourself while you dial it in.",
  },
];

/** First-run tutorial: one card per concept, then a starting-point choice. */
export function OnboardingModal() {
  const show = useMixerStore((s) => s.showOnboarding);
  const replay = useMixerStore((s) => s.onboardingReplay);
  const finishOnboarding = useMixerStore((s) => s.finishOnboarding);
  const [step, setStep] = useState(0);

  // Replays start from the first card again.
  useEffect(() => {
    if (show) setStep(0);
  }, [show]);

  if (!show) return null;

  const last = step === STEPS.length; // the choice page
  const current = STEPS[step];

  return (
    <div className="modal-scrim">
      <div className="modal ob-modal" role="dialog" aria-label="Welcome to Sink">
        {last && replay ? (
          <>
            <div className="modal-title">That's the tour</div>
            <p className="modal-text">
              Channels, apps and the mic are all live — your setup is
              untouched.
            </p>
            <div className="modal-btns">
              <button
                className="modal-btn primary"
                onClick={() => void finishOnboarding(false)}
              >
                Done
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
        ) : last ? (
          <>
            <div className="modal-title">How do you want to start?</div>
            <p className="modal-text">
              Either way you can add, rename or delete channels whenever —
              this just lays out your first board.
            </p>
            <div className="ob-choices">
              <button className="ob-choice" onClick={() => void finishOnboarding(false)}>
                <Ms name="dashboard" />
                <div className="ob-choice-title">Set up a board for me</div>
                <div className="ob-choice-sub">
                  Game, Chat, Music and System — ready to drop apps onto
                </div>
              </button>
              <button className="ob-choice" onClick={() => void finishOnboarding(true)}>
                <Ms name="check_box_outline_blank" />
                <div className="ob-choice-title">I'll build my own</div>
                <div className="ob-choice-sub">
                  One Main channel — add the rest as you go
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
            {current.diagram && <pre className="ob-diagram">{current.diagram}</pre>}
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
