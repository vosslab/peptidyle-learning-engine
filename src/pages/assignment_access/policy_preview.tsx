// policy_preview.tsx - server-derived preview with safe display-only Assignment Policy Sources.

import { For, Match, Show, Switch, type JSX } from "solid-js";

import type { TeachingPreviewView } from "../../../generated/api/TeachingPreviewView";
import { assignmentPolicySourceLabel, startLabel } from "./model";

export interface PolicyPreviewProps {
  readonly preview: TeachingPreviewView | undefined;
  readonly loading: boolean;
  readonly failure: string;
}

function displayTime(value: string | null): string {
  return value === null ? "Unrestricted" : value;
}

function displayLimit(value: number | null): string {
  return value === null ? "Unrestricted" : String(value);
}

function displayStart(
  preview: Extract<TeachingPreviewView, { active_student_course_membership: "allowed" }>,
): string {
  const label = startLabel(preview.start.kind);
  const result = preview.start.kind === "mayStart" ? `${label} (${preview.start.late})` : label;
  return result;
}

export function PolicyPreview(props: PolicyPreviewProps): JSX.Element {
  const allowed = ():
    Extract<TeachingPreviewView, { active_student_course_membership: "allowed" }> | undefined => {
    const preview = props.preview;
    return preview?.active_student_course_membership === "allowed" ? preview : undefined;
  };
  return (
    <section
      class="assignment-access-preview"
      aria-labelledby="assignment-access-preview-heading"
      aria-busy={props.loading}
    >
      <h2 id="assignment-access-preview-heading">Resolved Student preview</h2>
      <Switch fallback={<p>Select a Student to request a server-derived preview.</p>}>
        <Match when={props.loading}>
          <p role="status">Resolving access...</p>
        </Match>
        <Match when={props.failure}>
          <p role="alert" class="assignment-access-error">
            {props.failure}
          </p>
        </Match>
        <Match when={props.preview?.active_student_course_membership === "denied"}>
          <p role="status">This Student does not currently have access to this assignment.</p>
        </Match>
        <Match when={allowed()}>
          <Show when={allowed()}>
            {(preview) => (
              <>
                <p class="assignment-access-time-zone">
                  Course time zone: <strong>{preview().timeZone}</strong>
                </p>
                <dl class="assignment-access-preview-grid">
                  <dt>Start</dt>
                  <dd>{displayStart(preview())}</dd>
                  <dt>Available</dt>
                  <dd>{displayTime(preview().available_at.value)}</dd>
                  <dt>Due</dt>
                  <dd>{displayTime(preview().due_at.value)}</dd>
                  <dt>Closes</dt>
                  <dd>{displayTime(preview().closes_at.value)}</dd>
                  <dt>Whole Assignment Attempt seconds</dt>
                  <dd>{displayLimit(preview().assignment_attempt_time_limit_seconds.value)}</dd>
                  <dt>Attempt limit</dt>
                  <dd>{displayLimit(preview().attempt_limit.value)}</dd>
                  <dt>Late work</dt>
                  <dd>{preview().late_work_rule.value}</dd>
                  <dt>Deadline behavior</dt>
                  <dd>{preview().assignment_deadline_rule.value}</dd>
                </dl>
                <details>
                  <summary>Assignment Policy Sources</summary>
                  <ol class="assignment-access-policy-sources">
                    <For
                      each={
                        [
                          ["Available", preview().available_at.source],
                          ["Due", preview().due_at.source],
                          ["Closes", preview().closes_at.source],
                          [
                            "Whole Assignment Attempt seconds",
                            preview().assignment_attempt_time_limit_seconds.source,
                          ],
                          ["Attempt limit", preview().attempt_limit.source],
                          ["Late work", preview().late_work_rule.source],
                          ["Deadline behavior", preview().assignment_deadline_rule.source],
                        ] as const
                      }
                    >
                      {([label, source]) => (
                        <li>
                          <strong>{label}:</strong> {assignmentPolicySourceLabel(source)}
                        </li>
                      )}
                    </For>
                  </ol>
                </details>
              </>
            )}
          </Show>
        </Match>
      </Switch>
    </section>
  );
}
