// curriculum_adoption_panels.tsx - proposal, recovery, import, and receipt evidence panels.

import { A } from "@solidjs/router";
import { For, Show, type JSX } from "solid-js";

import type { AlphaInstantiationPreviewView } from "../../../generated/api/AlphaInstantiationPreviewView";
import type { AssignmentFastForwardPreviewView } from "../../../generated/api/AssignmentFastForwardPreviewView";
import type { BlueprintInstantiationPreviewView } from "../../../generated/api/BlueprintInstantiationPreviewView";
import type { CourseRolloverPreviewView } from "../../../generated/api/CourseRolloverPreviewView";
import type { CourseTermShiftPreviewOutcome } from "../../../generated/api/CourseTermShiftPreviewOutcome";
import type { CourseReference } from "../../../generated/api/CourseReference";
import type { CurriculumAdoptionReconciliationResult } from "../../../generated/api/CurriculumAdoptionReconciliationResult";
import type { CurriculumAdoptionReceiptBinding } from "../../../generated/api/CurriculumAdoptionReceiptBinding";
import type { CurriculumPinReplacement } from "../../../generated/api/CurriculumPinReplacement";
import type { CurriculumScheduleCorrection } from "../../../generated/api/CurriculumScheduleCorrection";
import type { PreparedCurriculumAssignmentView } from "../../../generated/api/PreparedCurriculumAssignmentView";
import type { PreparedCurriculumCourseView } from "../../../generated/api/PreparedCurriculumCourseView";
import type { SourceDerivedAssignmentPreviewView } from "../../../generated/api/SourceDerivedAssignmentPreviewView";
import type { UnavailablePinRecoveryAction } from "../../../generated/api/UnavailablePinRecoveryAction";
import type { CurriculumAdoptionClient } from "../../api/curriculum_adoption";
import { courseRouteReference } from "../../navigation/public_route";

export type CurriculumAdoptionPreview =
  | { readonly kind: "blueprint"; readonly value: BlueprintInstantiationPreviewView }
  | { readonly kind: "alpha"; readonly value: AlphaInstantiationPreviewView }
  | { readonly kind: "rollover"; readonly value: CourseRolloverPreviewView }
  | {
      readonly kind: "termShift";
      readonly value: Extract<CourseTermShiftPreviewOutcome, { kind: "eligible" }>;
    }
  | { readonly kind: "fastForward"; readonly value: AssignmentFastForwardPreviewView }
  | { readonly kind: "sourceDerived"; readonly value: SourceDerivedAssignmentPreviewView };

function preparedAssignments(
  preview: CurriculumAdoptionPreview,
): ReadonlyArray<PreparedCurriculumAssignmentView> {
  switch (preview.kind) {
    case "blueprint":
      return [preview.value.assignment];
    case "alpha":
    case "rollover":
      return preview.value.course.assignments;
    case "termShift":
      return preview.value.preview.assignments;
    case "sourceDerived":
      return [preview.value.assignment];
    case "fastForward":
      return [];
  }
}

function preparedCourse(
  preview: CurriculumAdoptionPreview,
): PreparedCurriculumCourseView | undefined {
  return preview.kind === "alpha" || preview.kind === "rollover" ? preview.value.course : undefined;
}

function previewCorrections(
  preview: CurriculumAdoptionPreview,
): ReadonlyArray<CurriculumScheduleCorrection> {
  switch (preview.kind) {
    case "blueprint":
    case "alpha":
    case "rollover":
    case "sourceDerived":
      return preview.value.corrections;
    case "termShift":
      return preview.value.preview.corrections;
    case "fastForward":
      return [];
  }
}

function previewPinCorrection(
  preview: CurriculumAdoptionPreview,
): UnavailablePinRecoveryAction | null {
  switch (preview.kind) {
    case "blueprint":
    case "alpha":
    case "rollover":
    case "sourceDerived":
      return preview.value.pinCorrection;
    case "termShift":
    case "fastForward":
      return null;
  }
}

export function previewNeedsRecovery(preview: CurriculumAdoptionPreview): boolean {
  return previewCorrections(preview).length > 0 || previewPinCorrection(preview) !== null;
}

export function replaceCurriculumPin(
  replacements: ReadonlyArray<CurriculumPinReplacement>,
  action: UnavailablePinRecoveryAction,
  question: string,
): ReadonlyArray<CurriculumPinReplacement> {
  const samePosition = (candidate: CurriculumPinReplacement): boolean =>
    candidate.position.moduleIndex === action.position.moduleIndex &&
    candidate.position.assignmentIndex === action.position.assignmentIndex &&
    candidate.position.entryIndex === action.position.entryIndex &&
    candidate.position.candidateIndex === action.position.candidateIndex;
  const replacement: CurriculumPinReplacement = { position: action.position, question };
  return [...replacements.filter((candidate) => !samePosition(candidate)), replacement];
}

export function PreviewPanel(props: {
  readonly preview: CurriculumAdoptionPreview | undefined;
  readonly onBack: () => void;
  readonly onApply: () => void;
  readonly onSourceDerived: () => void;
}): JSX.Element {
  if (props.preview?.kind === "fastForward") {
    const decision = props.preview.value.decision;
    return (
      <section
        class="curriculum-adoption-preview"
        aria-label="Server-owned controlled-update decision"
      >
        <h2>Review the controlled update</h2>
        <p>
          {decision.kind === "eligible"
            ? "The imported assignment can safely fast-forward to its observed source revision."
            : "The server preserved the current assignment and selected a recovery path."}
        </p>
        <div class="curriculum-adoption-actions">
          <button type="button" onClick={props.onBack}>
            Return to course changes
          </button>
          <Show
            when={decision.kind === "eligible"}
            fallback={
              <Show when={decision.kind === "divergent" || decision.kind === "issuedWork"}>
                <button
                  class="primary-action curriculum-adoption-primary"
                  type="button"
                  onClick={props.onSourceDerived}
                >
                  Create new assignment from this source definition
                </button>
              </Show>
            }
          >
            <button
              class="primary-action curriculum-adoption-primary"
              type="button"
              onClick={props.onApply}
            >
              Apply controlled update
            </button>
          </Show>
        </div>
      </section>
    );
  }
  return (
    <section class="curriculum-adoption-preview" aria-label="Server-owned curriculum proposal">
      <h2>Review the proposal</h2>
      <Show
        when={props.preview}
        fallback={
          <p role="alert">
            The prepared proposal is unavailable. Return to choices and prepare it again.
          </p>
        }
      >
        {(current) => (
          <>
            <Show when={preparedCourse(current())}>
              {(course) => (
                <p>
                  New live course: <strong>{course().title}</strong>
                </p>
              )}
            </Show>
            <ul class="curriculum-adoption-manifest" aria-label="Prepared assignments">
              <For each={preparedAssignments(current())}>
                {(assignment) => (
                  <li>
                    <strong>{assignment.title}</strong>
                    <span>{assignment.schedule.timeZone} schedule prepared by the server.</span>
                  </li>
                )}
              </For>
            </ul>
            <div class="curriculum-adoption-actions">
              <button type="button" onClick={props.onBack}>
                Change choices
              </button>
              <button
                class="primary-action curriculum-adoption-primary"
                type="button"
                onClick={props.onApply}
              >
                Apply live change
              </button>
            </div>
          </>
        )}
      </Show>
    </section>
  );
}

export function RecoveryPanel(props: {
  readonly preview: CurriculumAdoptionPreview | undefined;
  readonly replacements: ReadonlyArray<CurriculumPinReplacement>;
  readonly onChooseReplacement: (action: UnavailablePinRecoveryAction, question: string) => void;
  readonly onRegenerate: () => void;
}): JSX.Element {
  const correction = (): UnavailablePinRecoveryAction | null =>
    props.preview === undefined ? null : previewPinCorrection(props.preview);
  return (
    <section class="curriculum-adoption-recovery" aria-label="Proposal recovery">
      <h2>Resolve the proposal blocker</h2>
      <Show
        when={props.preview}
        fallback={<p>Reload the source or course, then prepare a new proposal.</p>}
      >
        {(current) => (
          <>
            <For each={previewCorrections(current())}>
              {(item) => <p>{item.correction.message}</p>}
            </For>
            <Show when={correction()}>
              {(action) => (
                <fieldset>
                  <legend>Replacement question</legend>
                  <p>Choose one valid replacement, then regenerate the proposal.</p>
                  <div class="curriculum-adoption-inline-actions">
                    <For each={action().candidates}>
                      {(question) => (
                        <button
                          type="button"
                          onClick={() => props.onChooseReplacement(action(), question)}
                        >
                          {question}
                        </button>
                      )}
                    </For>
                  </div>
                </fieldset>
              )}
            </Show>
          </>
        )}
      </Show>
      <div class="curriculum-adoption-actions">
        <span>
          {props.replacements.length > 0
            ? "A replacement is preserved for the next preview."
            : "Keep the existing source selections, then refresh the proposal."}
        </span>
        <button
          class="primary-action curriculum-adoption-primary"
          type="button"
          onClick={props.onRegenerate}
        >
          Regenerate proposal
        </button>
      </div>
    </section>
  );
}

export function ImportInspection(props: {
  readonly inspection:
    Awaited<ReturnType<CurriculumAdoptionClient["inspectCurriculumImports"]>> | undefined;
  readonly onBack: () => void;
  readonly onFastForward: (assignment: string) => void;
}): JSX.Element {
  return (
    <section class="curriculum-adoption-imports" aria-label="Curriculum import evidence">
      <h2>Imported curriculum evidence</h2>
      <Show
        when={props.inspection}
        fallback={
          <p role="alert">
            Import evidence is unavailable. Return to the course choices and retry.
          </p>
        }
      >
        {(inspection) => (
          <>
            <p>
              Origin: {inspection().origin.kind}. Schedule revision is current for this inspection.
            </p>
            <ul class="curriculum-adoption-import-list">
              <For each={inspection().assignments}>
                {(item) => (
                  <li>
                    <strong>{item.assignment}</strong>
                    <span>
                      {item.reusableMeaningMatchesBaseline
                        ? "Matches its imported baseline."
                        : "Has diverged from its imported baseline; preserve it and create a new source-derived assignment when the server offers that recovery."}
                    </span>
                    <Show when={item.source.kind === "reusable"}>
                      <button type="button" onClick={() => props.onFastForward(item.assignment)}>
                        Preview controlled update
                      </button>
                    </Show>
                  </li>
                )}
              </For>
            </ul>
            <div class="curriculum-adoption-actions">
              <button type="button" onClick={props.onBack}>
                Return to course changes
              </button>
            </div>
          </>
        )}
      </Show>
    </section>
  );
}

export function ReceiptPanel(props: {
  readonly courseReference: CourseReference;
  readonly receipt: CurriculumAdoptionReceiptBinding | undefined;
  readonly reconciliation: CurriculumAdoptionReconciliationResult | undefined;
  readonly onInspect: () => void;
}): JSX.Element {
  const reference = courseRouteReference(props.courseReference);
  return (
    <section class="curriculum-adoption-receipt" aria-label="Completed curriculum adoption">
      <h2>Live change complete</h2>
      <p>
        The server recorded an immutable receipt for this adopted curriculum. Its idempotency
        binding is retained privately by the browser contract.
      </p>
      <Show when={props.reconciliation}>
        {(result) => (
          <p role="status">
            {result().kind === "alreadyConsistent"
              ? "Import projections are already consistent."
              : "Import projections were repaired from immutable evidence."}
          </p>
        )}
      </Show>
      <div class="curriculum-adoption-receipt-actions">
        <A class="primary-link" href={`/courses/${reference}`}>
          Open course
        </A>
        <A href={`/instructor/courses/${reference}/curriculum`}>Inspect imports</A>
        <button type="button" onClick={props.onInspect} disabled={props.receipt === undefined}>
          Check receipt evidence
        </button>
      </div>
    </section>
  );
}
