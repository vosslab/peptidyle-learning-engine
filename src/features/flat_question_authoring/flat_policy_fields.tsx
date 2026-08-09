// flat_policy_fields.tsx - explicit scoring, attempt, disclosure, and timing policy controls.

import { Show, type JSX } from "solid-js";

import type {
  FlatQuestionAttemptPolicy,
  FlatQuestionFeedbackDisclosure,
  FlatQuestionTimingPolicy,
} from "./flat_question_source";

const DISCLOSURES: ReadonlyArray<{
  readonly value: FlatQuestionFeedbackDisclosure;
  readonly label: string;
}> = [
  { value: "immediateFull", label: "Immediate full feedback" },
  { value: "immediateCorrectness", label: "Immediate correctness only" },
  { value: "deferred", label: "Deferred feedback" },
  { value: "onRelease", label: "Feedback on release" },
];

export interface FlatPolicyFieldsProps {
  readonly points: number;
  readonly attemptPolicy: FlatQuestionAttemptPolicy;
  readonly timingPolicy: FlatQuestionTimingPolicy;
  readonly onPointsChange: (points: number) => void;
  readonly onAttemptPolicyChange: (policy: FlatQuestionAttemptPolicy) => void;
  readonly onTimingPolicyChange: (policy: FlatQuestionTimingPolicy) => void;
  readonly fieldErrors?: Readonly<Record<string, string | undefined>>;
  readonly disabled?: boolean;
}

function finiteNumber(value: string, fallback: number): number {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function positiveInteger(value: string, fallback: number): number {
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed >= 1 ? parsed : fallback;
}

function nonnegativeInteger(value: string, fallback: number): number {
  const parsed = Number(value);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? parsed : fallback;
}

function isDisclosure(value: string): value is FlatQuestionFeedbackDisclosure {
  return DISCLOSURES.some((disclosure) => disclosure.value === value);
}

function isTimingKind(value: string): value is FlatQuestionTimingPolicy["kind"] {
  return value === "untimed" || value === "perQuestion" || value === "perAttempt";
}

/** Policy edits use native controls and keep unlimited attempts as a visible, reversible choice. */
export function FlatPolicyFields(props: FlatPolicyFieldsProps): JSX.Element {
  const timingKind = (): FlatQuestionTimingPolicy["kind"] => props.timingPolicy.kind;
  const timingSeconds = (): number =>
    props.timingPolicy.kind === "untimed" ? 60 : props.timingPolicy.seconds;
  const graceSeconds = (): number =>
    props.timingPolicy.kind === "untimed" ? 0 : props.timingPolicy.graceSeconds;
  const error =
    (field: string): (() => string | undefined) =>
    (): string | undefined =>
      props.fieldErrors?.[field];
  const setTimingKind = (kind: FlatQuestionTimingPolicy["kind"]): void => {
    if (kind === "untimed") {
      props.onTimingPolicyChange({ kind });
      return;
    }
    props.onTimingPolicyChange({ kind, seconds: timingSeconds(), graceSeconds: graceSeconds() });
  };
  const setTimingValue = (field: "seconds" | "graceSeconds", value: string): void => {
    const next =
      field === "seconds"
        ? positiveInteger(value, timingSeconds())
        : nonnegativeInteger(value, graceSeconds());
    const kind = timingKind();
    if (kind === "untimed") return;
    props.onTimingPolicyChange({ ...props.timingPolicy, [field]: next });
  };
  return (
    <fieldset>
      <legend>Scoring and response policy</legend>
      <div class="flat-question-authoring__grid">
        <label class="flat-question-authoring__field">
          <span>Points</span>
          <input
            type="number"
            min="0"
            step="0.01"
            value={props.points}
            disabled={props.disabled}
            aria-invalid={error("points")() !== undefined}
            aria-describedby={error("points")() === undefined ? undefined : "flat-points-error"}
            onInput={(event) =>
              props.onPointsChange(finiteNumber(event.currentTarget.value, props.points))
            }
          />
        </label>
        <label class="flat-question-authoring__field">
          <span>Maximum attempts</span>
          <input
            type="number"
            min="1"
            step="1"
            disabled={props.disabled || props.attemptPolicy.maxAttempts === null}
            value={props.attemptPolicy.maxAttempts ?? ""}
            aria-invalid={error("attemptPolicy.maxAttempts")() !== undefined}
            aria-describedby={
              error("attemptPolicy.maxAttempts")() === undefined ? undefined : "flat-attempts-error"
            }
            onInput={(event) =>
              props.onAttemptPolicyChange({
                ...props.attemptPolicy,
                maxAttempts: positiveInteger(event.currentTarget.value, 1),
              })
            }
          />
        </label>
      </div>
      <Show when={error("points")() !== undefined}>
        <p class="flat-question-authoring__error" id="flat-points-error" role="alert">
          {error("points")()}
        </p>
      </Show>
      <Show when={error("attemptPolicy.maxAttempts")() !== undefined}>
        <p class="flat-question-authoring__error" id="flat-attempts-error" role="alert">
          {error("attemptPolicy.maxAttempts")()}
        </p>
      </Show>
      <label class="flat-question-authoring__field">
        <span>
          <input
            type="checkbox"
            checked={props.attemptPolicy.maxAttempts === null}
            disabled={props.disabled}
            onChange={(event) =>
              props.onAttemptPolicyChange({
                ...props.attemptPolicy,
                maxAttempts: event.currentTarget.checked ? null : 1,
              })
            }
          />{" "}
          Allow unlimited attempts
        </span>
      </label>
      <label class="flat-question-authoring__field">
        <span>Feedback disclosure</span>
        <select
          value={props.attemptPolicy.feedback}
          disabled={props.disabled}
          onChange={(event) => {
            const feedback = event.currentTarget.value;
            if (isDisclosure(feedback))
              props.onAttemptPolicyChange({ ...props.attemptPolicy, feedback });
          }}
        >
          {DISCLOSURES.map((disclosure) => (
            <option value={disclosure.value}>{disclosure.label}</option>
          ))}
        </select>
      </label>
      <fieldset>
        <legend>Timing</legend>
        <label class="flat-question-authoring__field">
          <span>Timing mode</span>
          <select
            value={timingKind()}
            disabled={props.disabled}
            onChange={(event) => {
              const kind = event.currentTarget.value;
              if (isTimingKind(kind)) setTimingKind(kind);
            }}
          >
            <option value="untimed">Untimed</option>
            <option value="perQuestion">Time for the question</option>
            <option value="perAttempt">Time for each attempt</option>
          </select>
        </label>
        <Show when={timingKind() !== "untimed"}>
          <div class="flat-question-authoring__grid">
            <label class="flat-question-authoring__field">
              <span>Seconds</span>
              <input
                type="number"
                min="1"
                step="1"
                value={timingSeconds()}
                disabled={props.disabled}
                onInput={(event) => setTimingValue("seconds", event.currentTarget.value)}
              />
            </label>
            <label class="flat-question-authoring__field">
              <span>Grace seconds</span>
              <input
                type="number"
                min="0"
                step="1"
                value={graceSeconds()}
                disabled={props.disabled}
                onInput={(event) => setTimingValue("graceSeconds", event.currentTarget.value)}
              />
            </label>
          </div>
        </Show>
      </fieldset>
    </fieldset>
  );
}
