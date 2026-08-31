// policy_preview.tsx - server-derived preview with safe display-only provenance.

import { For, Match, Show, Switch, type JSX } from "solid-js";

import type { TeachingPreviewView } from "../../../generated/api/TeachingPreviewView";
import { sourceLabel, startLabel } from "./model";

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

function displayStart(preview: Extract<TeachingPreviewView, { entitlement: "allowed" }>): string {
  const label = startLabel(preview.start.kind);
  const result = preview.start.kind === "mayStart" ? `${label} (${preview.start.late})` : label;
  return result;
}

export function PolicyPreview(props: PolicyPreviewProps): JSX.Element {
  const allowed = (): Extract<TeachingPreviewView, { entitlement: "allowed" }> | undefined => {
    const preview = props.preview;
    return preview?.entitlement === "allowed" ? preview : undefined;
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
        <Match when={props.preview?.entitlement === "denied"}>
          <p role="status">This Student is not entitled to this assignment.</p>
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
                  <dd>{displayTime(preview().availableAt.value)}</dd>
                  <dt>Due</dt>
                  <dd>{displayTime(preview().dueAt.value)}</dd>
                  <dt>Closes</dt>
                  <dd>{displayTime(preview().closesAt.value)}</dd>
                  <dt>Whole-run seconds</dt>
                  <dd>{displayLimit(preview().timeLimitSeconds.value)}</dd>
                  <dt>Attempt limit</dt>
                  <dd>{displayLimit(preview().attemptLimit.value)}</dd>
                  <dt>Late work</dt>
                  <dd>{preview().lateSubmission.value}</dd>
                  <dt>Deadline behavior</dt>
                  <dd>{preview().deadlineBehavior.value}</dd>
                </dl>
                <details>
                  <summary>Field provenance</summary>
                  <ol class="assignment-access-provenance">
                    <For
                      each={
                        [
                          ["Available", preview().availableAt.source],
                          ["Due", preview().dueAt.source],
                          ["Closes", preview().closesAt.source],
                          ["Whole-run seconds", preview().timeLimitSeconds.source],
                          ["Attempt limit", preview().attemptLimit.source],
                          ["Late work", preview().lateSubmission.source],
                          ["Deadline behavior", preview().deadlineBehavior.source],
                        ] as const
                      }
                    >
                      {([label, source]) => (
                        <li>
                          <strong>{label}:</strong> {sourceLabel(source)}
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
