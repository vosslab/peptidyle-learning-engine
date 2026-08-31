// flat_policy_fields.tsx - explicit scoring, retry, and timing policy controls.

import { Show, type JSX } from "solid-js";

import type { FlatQuestionAttemptLimit, FlatQuestionAttemptTimeLimit } from "./flat_question_source";

export interface FlatPolicyFieldsProps {
  readonly points: number;
  readonly questionAttemptLimit: FlatQuestionAttemptLimit;
  readonly questionAttemptTimeLimit: FlatQuestionAttemptTimeLimit;
  readonly onPointsChange: (points: number) => void;
  readonly onQuestionAttemptLimitChange: (limit: FlatQuestionAttemptLimit) => void;
  readonly onQuestionAttemptTimeLimitChange: (policy: FlatQuestionAttemptTimeLimit) => void;
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

function isTimingKind(value: string): value is FlatQuestionAttemptTimeLimit["kind"] {
  return value === "unlimited" || value === "limited";
}

/** Retry and timing edits use native controls and keep unlimited attempts reversible. */
export function FlatPolicyFields(props: FlatPolicyFieldsProps): JSX.Element {
  const timingKind = (): FlatQuestionAttemptTimeLimit["kind"] => props.questionAttemptTimeLimit.kind;
  const timingSeconds = (): number =>
    props.questionAttemptTimeLimit.kind === "unlimited" ? 60 : props.questionAttemptTimeLimit.seconds;
  const graceSeconds = (): number =>
    props.questionAttemptTimeLimit.kind === "unlimited" ? 0 : props.questionAttemptTimeLimit.graceSeconds;
  const error =
    (field: string): (() => string | undefined) =>
    (): string | undefined =>
      props.fieldErrors?.[field];
  const setTimingKind = (kind: FlatQuestionAttemptTimeLimit["kind"]): void => {
    if (kind === "unlimited") {
      props.onQuestionAttemptTimeLimitChange({ kind });
      return;
    }
    props.onQuestionAttemptTimeLimitChange({ kind, seconds: timingSeconds(), graceSeconds: graceSeconds() });
  };
  const setTimingValue = (field: "seconds" | "graceSeconds", value: string): void => {
    const next =
      field === "seconds"
        ? positiveInteger(value, timingSeconds())
        : nonnegativeInteger(value, graceSeconds());
    const kind = timingKind();
    if (kind === "unlimited") return;
    props.onQuestionAttemptTimeLimitChange({ ...props.questionAttemptTimeLimit, [field]: next });
  };
  return (
    <fieldset>
      <legend>Scoring, retries, and timing</legend>
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
            disabled={props.disabled || props.questionAttemptLimit.maxAttempts === null}
            value={props.questionAttemptLimit.maxAttempts ?? ""}
            aria-invalid={error("questionAttemptLimit.maxAttempts")() !== undefined}
            aria-describedby={
              error("questionAttemptLimit.maxAttempts")() === undefined
                ? undefined
                : "flat-attempts-error"
            }
            onInput={(event) =>
              props.onQuestionAttemptLimitChange({
                ...props.questionAttemptLimit,
                maxAttempts: positiveInteger(event.currentTarget.value, 1),
              })
            }
          />
          <small class="flat-question-authoring__help">
            This controls retries only. Assignment settings control what students can see.
          </small>
        </label>
      </div>
      <Show when={error("points")() !== undefined}>
        <p class="flat-question-authoring__error" id="flat-points-error" role="alert">
          {error("points")()}
        </p>
      </Show>
      <Show when={error("questionAttemptLimit.maxAttempts")() !== undefined}>
        <p class="flat-question-authoring__error" id="flat-attempts-error" role="alert">
          {error("questionAttemptLimit.maxAttempts")()}
        </p>
      </Show>
      <label class="flat-question-authoring__field">
        <span>
          <input
            type="checkbox"
            checked={props.questionAttemptLimit.maxAttempts === null}
            disabled={props.disabled}
            onChange={(event) =>
              props.onQuestionAttemptLimitChange({
                ...props.questionAttemptLimit,
                maxAttempts: event.currentTarget.checked ? null : 1,
              })
            }
          />{" "}
          Allow unlimited attempts
        </span>
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
            <option value="unlimited">No Question Attempt time limit</option>
            <option value="limited">Question Attempt time limit</option>
          </select>
        </label>
        <Show when={timingKind() !== "unlimited"}>
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
