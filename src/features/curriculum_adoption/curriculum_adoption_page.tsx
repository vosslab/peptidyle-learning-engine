// curriculum_adoption_page.tsx - visible, server-owned proposals for live teaching-course changes.

import { A } from "@solidjs/router";
import { For, Match, Show, Switch, createMemo, createSignal, onMount, type JSX } from "solid-js";

import type { AlphaCourseSummaryView } from "../../../generated/api/AlphaCourseSummaryView";
import type { AlphaInstantiationPreviewView } from "../../../generated/api/AlphaInstantiationPreviewView";
import type { AssignmentFastForwardPreviewView } from "../../../generated/api/AssignmentFastForwardPreviewView";
import type { BlueprintInstantiationPreviewView } from "../../../generated/api/BlueprintInstantiationPreviewView";
import type { BlueprintSummaryView } from "../../../generated/api/BlueprintSummaryView";
import type { CourseRolloverPreviewView } from "../../../generated/api/CourseRolloverPreviewView";
import type { CourseTermShiftPreviewOutcome } from "../../../generated/api/CourseTermShiftPreviewOutcome";
import type { CourseSummary } from "../../../generated/api/CourseSummary";
import type { CourseTerm } from "../../../generated/api/CourseTerm";
import type { CurriculumAdoptionReconciliationResult } from "../../../generated/api/CurriculumAdoptionReconciliationResult";
import type { CurriculumPinReplacement } from "../../../generated/api/CurriculumPinReplacement";
import type { CurriculumScheduleCorrection } from "../../../generated/api/CurriculumScheduleCorrection";
import type { PreparedCurriculumAssignmentView } from "../../../generated/api/PreparedCurriculumAssignmentView";
import type { PreparedCurriculumCourseView } from "../../../generated/api/PreparedCurriculumCourseView";
import type { SourceDerivedAssignmentPreviewView } from "../../../generated/api/SourceDerivedAssignmentPreviewView";
import type { UnavailablePinRecoveryAction } from "../../../generated/api/UnavailablePinRecoveryAction";
import type {
  CurriculumAdoptionClient,
  CurriculumAdoptionCompleted,
  EligibleAssignmentFastForwardPreview,
} from "../../api/curriculum_adoption";
import type { ReusableCurriculumClient } from "../../api/reusable_curriculum";
import {
  curriculumAdoptionNextInstruction,
  curriculumAdoptionOperationNeedsTerm,
  curriculumAdoptionOperationNeedsTitle,
  curriculumAdoptionOperationPresentation,
  withCurriculumAdoptionOperation,
  withCurriculumAdoptionSource,
  type CurriculumAdoptionOperation,
  type CurriculumAdoptionStage,
} from "./curriculum_adoption_model";
import { ReceiptPanel } from "./curriculum_adoption_panels";
import "./curriculum_adoption_page.css";

type SourceSelection =
  | { readonly kind: "blueprint"; readonly value: BlueprintSummaryView }
  | { readonly kind: "alpha"; readonly value: AlphaCourseSummaryView };

type Preview =
  | { readonly kind: "blueprint"; readonly value: BlueprintInstantiationPreviewView }
  | { readonly kind: "alpha"; readonly value: AlphaInstantiationPreviewView }
  | { readonly kind: "rollover"; readonly value: CourseRolloverPreviewView }
  | {
      readonly kind: "termShift";
      readonly value: Extract<CourseTermShiftPreviewOutcome, { kind: "eligible" }>;
    }
  | { readonly kind: "fastForward"; readonly value: AssignmentFastForwardPreviewView }
  | { readonly kind: "sourceDerived"; readonly value: SourceDerivedAssignmentPreviewView };

type CourseLocalCompleted = Extract<CurriculumAdoptionCompleted, { readonly course: unknown }>;

type Notice = { readonly kind: "status" | "alert"; readonly text: string };

export interface CurriculumAdoptionPageProps {
  readonly course: CourseSummary;
  readonly client: CurriculumAdoptionClient;
  readonly reusableClient: ReusableCurriculumClient;
}

function newIdempotencyKey(): string {
  return globalThis.crypto.randomUUID();
}

function errorMessage(error: unknown, fallback: string): string {
  if (error instanceof Error && error.message.length > 0) return error.message;
  return fallback;
}

function sourceLabel(source: SourceSelection): string {
  return source.kind === "blueprint" ? source.value.title : source.value.title;
}

function preparedAssignments(preview: Preview): ReadonlyArray<PreparedCurriculumAssignmentView> {
  switch (preview.kind) {
    case "blueprint":
      return [preview.value.assignment];
    case "alpha":
      return preview.value.course.assignments;
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

function preparedCourse(preview: Preview): PreparedCurriculumCourseView | undefined {
  return preview.kind === "alpha" || preview.kind === "rollover" ? preview.value.course : undefined;
}

function previewCorrections(preview: Preview): ReadonlyArray<CurriculumScheduleCorrection> {
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

function previewPinCorrection(preview: Preview): UnavailablePinRecoveryAction | null {
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

function previewNeedsRecovery(preview: Preview): boolean {
  return previewCorrections(preview).length > 0 || previewPinCorrection(preview) !== null;
}

function replacePin(
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

function operationRequiresSelectedSource(
  operation: CurriculumAdoptionOperation,
): "blueprint" | "alpha" | null {
  if (operation === "blueprint") return "blueprint";
  if (operation === "alpha") return "alpha";
  return null;
}

/**
 * An Instructor-facing, course-local adoption flow. It preserves selections through stale and
 * retry recovery; the only mutable course result comes from an explicit server-owned apply.
 */
export function CurriculumAdoptionPage(props: CurriculumAdoptionPageProps): JSX.Element {
  const [stage, setStage] = createSignal<CurriculumAdoptionStage>("choose");
  const [operation, setOperation] = createSignal<CurriculumAdoptionOperation>("blueprint");
  const [blueprints, setBlueprints] = createSignal<ReadonlyArray<BlueprintSummaryView>>([]);
  const [alphaCourses, setAlphaCourses] = createSignal<ReadonlyArray<AlphaCourseSummaryView>>([]);
  const [source, setSource] = createSignal<SourceSelection>();
  const [targetTerm, setTargetTerm] = createSignal<CourseTerm>(props.course.term);
  const [title, setTitle] = createSignal(`${props.course.title} next term`);
  const [replacements, setReplacements] = createSignal<ReadonlyArray<CurriculumPinReplacement>>([]);
  const [preview, setPreview] = createSignal<Preview>();
  const [completed, setCompleted] = createSignal<CourseLocalCompleted>();
  const [reconciliation, setReconciliation] =
    createSignal<CurriculumAdoptionReconciliationResult>();
  const [notice, setNotice] = createSignal<Notice>({
    kind: "status",
    text: curriculumAdoptionNextInstruction("choose", "blueprint"),
  });
  const [loadingSources, setLoadingSources] = createSignal(true);
  const [sourcesError, setSourcesError] = createSignal<string>();
  const [applyingKey, setApplyingKey] = createSignal<string>();
  const [importInspection, setImportInspection] =
    createSignal<Awaited<ReturnType<CurriculumAdoptionClient["inspectCurriculumImports"]>>>();
  const requiredSourceKind = createMemo(() => operationRequiresSelectedSource(operation()));
  const operationPresentation = createMemo(() =>
    curriculumAdoptionOperationPresentation(operation()),
  );
  const canPreview = createMemo(() => {
    const required = requiredSourceKind();
    if (operation() === "imports") return true;
    return required === null || source()?.kind === required;
  });

  function announce(nextStage: CurriculumAdoptionStage, kind: Notice["kind"] = "status"): void {
    setNotice({ kind, text: curriculumAdoptionNextInstruction(nextStage, operation()) });
  }

  async function loadSources(): Promise<void> {
    setLoadingSources(true);
    setSourcesError(undefined);
    try {
      const [blueprintPage, alphaPage] = await Promise.all([
        props.reusableClient.listBlueprints(undefined, 100),
        props.reusableClient.listAlphaCourses(undefined, 100),
      ]);
      setBlueprints(blueprintPage.items);
      setAlphaCourses(alphaPage.items);
      setNotice({
        kind: "status",
        text: "Choose a reusable source or a course operation, then prepare the live proposal.",
      });
    } catch (error: unknown) {
      const message = errorMessage(error, "Reusable curriculum sources could not load. Try again.");
      setSourcesError(message);
      setNotice({ kind: "alert", text: message });
    } finally {
      setLoadingSources(false);
    }
  }

  async function inspectImports(): Promise<void> {
    setStage("previewing");
    announce("previewing");
    try {
      setImportInspection(await props.client.inspectCurriculumImports(props.course.reference));
      setStage("preview");
      setNotice({
        kind: "status",
        text: "Review the imported assignment evidence. Choose a server-supported update from its row when available.",
      });
    } catch (error: unknown) {
      setStage("recovery");
      setNotice({
        kind: "alert",
        text: errorMessage(
          error,
          "Import evidence could not load. Retry inspection when the course is available.",
        ),
      });
    }
  }

  async function preparePreview(): Promise<void> {
    if (operation() === "imports") {
      await inspectImports();
      return;
    }
    const selected = source();
    const expected = requiredSourceKind();
    if (expected !== null && selected?.kind !== expected) {
      setStage("recovery");
      setNotice({
        kind: "alert",
        text: `Choose a ${expected === "blueprint" ? "Blueprint" : "public Alpha curriculum"} before preparing this proposal.`,
      });
      return;
    }
    const inspection = importInspection();
    if ((operation() === "rollover" || operation() === "termShift") && inspection === undefined) {
      setStage("recovery");
      setNotice({
        kind: "alert",
        text: "Inspect curriculum imports first so the server can provide the current schedule witness for this live proposal.",
      });
      return;
    }

    setStage("previewing");
    announce("previewing");
    try {
      let nextPreview: Preview | undefined;
      if (operation() === "blueprint" && selected?.kind === "blueprint") {
        nextPreview = {
          kind: "blueprint",
          value: await props.client.previewBlueprintInstantiation({
            source: { reference: selected.value.reference, revision: selected.value.revision },
            course: props.course.reference,
            targetTerm: targetTerm(),
            replacements: [...replacements()],
          }),
        };
      } else if (operation() === "alpha" && selected?.kind === "alpha") {
        nextPreview = {
          kind: "alpha",
          value: await props.client.previewAlphaInstantiation({
            source: { reference: selected.value.reference, revision: selected.value.revision },
            title: title(),
            targetTerm: targetTerm(),
            replacements: [...replacements()],
          }),
        };
      } else if (operation() === "rollover" && inspection !== undefined) {
        nextPreview = {
          kind: "rollover",
          value: await props.client.previewCourseRollover({
            witness: inspection.witness,
            title: title(),
            targetTerm: targetTerm(),
            replacements: [...replacements()],
          }),
        };
      } else if (operation() === "termShift" && inspection !== undefined) {
        const outcome = await props.client.previewCourseTermShift({
          witness: inspection.witness,
          targetTerm: targetTerm(),
        });
        if (outcome.kind === "ineligible") {
          setPreview(undefined);
          setStage("recovery");
          setNotice({
            kind: "alert",
            text: "This course already has issued learner work, so its term remains preserved. Choose rollover to create the next live course.",
          });
          return;
        }
        nextPreview = { kind: "termShift", value: outcome };
      }
      if (nextPreview === undefined)
        throw new Error("The selected source no longer matches this operation.");
      setPreview(nextPreview);
      if (previewNeedsRecovery(nextPreview)) {
        setStage("recovery");
        announce("recovery");
      } else {
        setStage("preview");
        announce("preview");
      }
    } catch (error: unknown) {
      setStage("recovery");
      setNotice({
        kind: "alert",
        text: errorMessage(
          error,
          "The proposal became stale or could not be prepared. Reload its source and try again.",
        ),
      });
    }
  }

  async function applyPreview(): Promise<void> {
    const current = preview();
    if (current === undefined || previewNeedsRecovery(current)) return;
    if (current.kind === "fastForward" && current.value.decision.kind !== "eligible") {
      setStage("recovery");
      setNotice({
        kind: "alert",
        text: "This import cannot fast-forward in place. Use its offered recovery action to preserve the current assignment.",
      });
      return;
    }
    const key = applyingKey() ?? newIdempotencyKey();
    setApplyingKey(key);
    setStage("applying");
    announce("applying");
    try {
      let completedResult: CourseLocalCompleted;
      if (current.kind === "blueprint") {
        completedResult = await props.client.applyBlueprintInstantiation(current.value, key);
      } else if (current.kind === "alpha") {
        completedResult = await props.client.applyAlphaInstantiation(current.value, key);
      } else if (current.kind === "rollover") {
        completedResult = await props.client.applyCourseRollover(current.value, key);
      } else if (current.kind === "termShift") {
        completedResult = await props.client.applyCourseTermShift(current.value, key);
      } else if (current.kind === "sourceDerived") {
        completedResult = await props.client.applySourceDerivedAssignment(current.value, key);
      } else {
        completedResult = await props.client.applyAssignmentFastForward(
          current.value as EligibleAssignmentFastForwardPreview,
          key,
        );
      }
      setCompleted(completedResult);
      setStage("receipt");
      announce("receipt");
    } catch (error: unknown) {
      setStage("recovery");
      setNotice({
        kind: "alert",
        text: `${errorMessage(error, "The live change could not be confirmed.")} Retry applies the same proposal with its existing idempotency key.`,
      });
    }
  }

  async function reconcileReceipt(): Promise<void> {
    const completedResult = completed();
    if (completedResult === undefined) return;
    try {
      setReconciliation(
        await props.client.reconcileCurriculumAdoption({ receipt: completedResult.receipt }),
      );
      setNotice({
        kind: "status",
        text: "The receipt has been checked against immutable adoption evidence. Open the destination or inspect its imports next.",
      });
    } catch (error: unknown) {
      setNotice({
        kind: "alert",
        text: errorMessage(
          error,
          "Receipt inspection could not finish. Try again from this completed operation.",
        ),
      });
    }
  }

  async function prepareFastForward(assignmentReference: string): Promise<void> {
    const inspection = importInspection();
    const imported = inspection?.assignments.find(
      (item) => item.assignment === assignmentReference,
    );
    const observed = inspection?.witness.assignmentRevisions.find(
      (item) => item.assignment === assignmentReference,
    );
    if (
      inspection === undefined ||
      imported === undefined ||
      observed === undefined ||
      imported.source.kind !== "reusable"
    ) {
      setStage("recovery");
      setNotice({
        kind: "alert",
        text: "This import no longer has the current source and assignment witnesses needed for a controlled update. Inspect imports again.",
      });
      return;
    }
    setStage("previewing");
    setNotice({ kind: "status", text: "Preparing the server-owned controlled-update decision." });
    try {
      setPreview({
        kind: "fastForward",
        value: await props.client.previewAssignmentFastForward({
          course: inspection.witness.course,
          assignment: observed,
          importRevision: imported.revision,
          source: imported.source.definition,
        }),
      });
      setStage("preview");
      setNotice({
        kind: "status",
        text: "Review the controlled-update decision. The current assignment stays preserved unless you explicitly apply an eligible proposal.",
      });
    } catch (error: unknown) {
      setStage("recovery");
      setNotice({
        kind: "alert",
        text: errorMessage(
          error,
          "The controlled-update decision could not be prepared. Inspect imports again to refresh witnesses.",
        ),
      });
    }
  }

  async function prepareSourceDerived(): Promise<void> {
    const inspection = importInspection();
    const current = preview();
    if (inspection === undefined || current?.kind !== "fastForward") return;
    setStage("previewing");
    setNotice({
      kind: "status",
      text: "Preparing a separate source-derived assignment while preserving the divergent import.",
    });
    try {
      setPreview({
        kind: "sourceDerived",
        value: await props.client.previewSourceDerivedAssignment({
          course: inspection.witness.course,
          source: current.value.source,
          replacements: [...replacements()],
        }),
      });
      setStage("preview");
      setNotice({
        kind: "status",
        text: "Review the separate source-derived assignment, then apply it to preserve the divergent assignment unchanged.",
      });
    } catch (error: unknown) {
      setStage("recovery");
      setNotice({
        kind: "alert",
        text: errorMessage(
          error,
          "The separate assignment proposal could not be prepared. Refresh imports and try again.",
        ),
      });
    }
  }

  function chooseOperation(nextOperation: CurriculumAdoptionOperation): void {
    setOperation(
      withCurriculumAdoptionOperation({ operation: operation(), source: source() }, nextOperation)
        .operation,
    );
    setPreview(undefined);
    setCompleted(undefined);
    setReconciliation(undefined);
    setApplyingKey(undefined);
    setStage("choose");
    setNotice({ kind: "status", text: curriculumAdoptionNextInstruction("choose", nextOperation) });
  }

  function chooseSource(nextSource: SourceSelection): void {
    setSource(
      withCurriculumAdoptionSource({ source: source(), replacements: replacements() }, nextSource)
        .source,
    );
    setPreview(undefined);
    setApplyingKey(undefined);
    setStage("choose");
    setNotice({
      kind: "status",
      text: `${sourceLabel(nextSource)} is selected. Set the target term, then prepare the server-owned proposal.`,
    });
  }

  function updateTerm(field: keyof CourseTerm, value: string): void {
    setTargetTerm((current) => ({ ...current, [field]: value }));
    setPreview(undefined);
    setNotice({
      kind: "status",
      text: "Target term updated. Prepare a fresh server-owned proposal when the dates and time zone are ready.",
    });
  }

  onMount(() => void loadSources());

  return (
    <main class="page curriculum-adoption-page" data-route-surface="curriculumAdoption">
      <header>
        <p class="eyebrow">Live course change</p>
        <h1>Adopt reusable curriculum</h1>
        <p class="page-lede">
          Reusable meaning stays at the source. This workflow proposes and applies a visible change
          to the live teaching state for {props.course.title}.
        </p>
      </header>
      <p
        class="curriculum-adoption-status"
        role={notice().kind}
        aria-live="polite"
        aria-atomic="true"
      >
        {notice().text}
      </p>

      <section class="curriculum-adoption-summary" aria-label="Current destination">
        <strong>Live destination: {props.course.title}</strong>
        <span>
          {props.course.term.startDate} to {props.course.term.endDate} ·{" "}
          {props.course.term.timeZone}
        </span>
      </section>

      <Switch>
        <Match when={stage() === "choose" || stage() === "recovery"}>
          <section class="curriculum-adoption-workflow" aria-label="Curriculum adoption choices">
            <form
              class="curriculum-adoption-form"
              onSubmit={(event) => {
                event.preventDefault();
                void preparePreview();
              }}
            >
              <fieldset>
                <legend>1. Choose the live teaching change</legend>
                <div class="curriculum-adoption-operation-grid">
                  <For each={["blueprint", "alpha", "rollover", "termShift", "imports"] as const}>
                    {(candidate) => {
                      const presentation = curriculumAdoptionOperationPresentation(candidate);
                      return (
                        <label class="curriculum-adoption-operation">
                          <input
                            type="radio"
                            name="curriculum-operation"
                            checked={operation() === candidate}
                            onChange={() => chooseOperation(candidate)}
                          />
                          <strong>{presentation.label}</strong>
                          <span>{presentation.description}</span>
                        </label>
                      );
                    }}
                  </For>
                </div>
              </fieldset>

              <Show when={requiredSourceKind() !== null}>
                <fieldset>
                  <legend>2. Select reusable meaning</legend>
                  <Show
                    when={loadingSources()}
                    fallback={
                      <Show
                        when={sourcesError() === undefined}
                        fallback={
                          <div role="alert">
                            <p>{sourcesError()}</p>
                            <button type="button" onClick={() => void loadSources()}>
                              Reload sources
                            </button>
                          </div>
                        }
                      >
                        <div class="curriculum-adoption-source-grid">
                          <For
                            each={
                              requiredSourceKind() === "blueprint" ? blueprints() : alphaCourses()
                            }
                          >
                            {(candidate) => {
                              const candidateSource: SourceSelection =
                                requiredSourceKind() === "blueprint"
                                  ? { kind: "blueprint", value: candidate as BlueprintSummaryView }
                                  : { kind: "alpha", value: candidate as AlphaCourseSummaryView };
                              return (
                                <label class="curriculum-adoption-source">
                                  <input
                                    type="radio"
                                    name="curriculum-source"
                                    checked={
                                      source()?.kind === candidateSource.kind &&
                                      source()?.value.reference === candidateSource.value.reference
                                    }
                                    onChange={() => chooseSource(candidateSource)}
                                  />
                                  <strong>{candidate.title}</strong>
                                  <span>
                                    {candidate.reference} · revision {candidate.revision}
                                  </span>
                                </label>
                              );
                            }}
                          </For>
                        </div>
                      </Show>
                    }
                  >
                    <p role="status">Loading live reusable sources…</p>
                  </Show>
                  <p class="curriculum-adoption-help">
                    Manage reusable sources in the <A href="/curriculum">curriculum workspace</A>.
                    Alpha forks remain source-owned there; this course page creates teaching state
                    only.
                  </p>
                </fieldset>
              </Show>

              <Show when={curriculumAdoptionOperationNeedsTerm(operation())}>
                <fieldset>
                  <legend>{requiredSourceKind() === null ? "2" : "3"}. Set the target term</legend>
                  <div class="curriculum-adoption-term-grid">
                    <label class="curriculum-adoption-field">
                      Start date
                      <input
                        type="date"
                        value={targetTerm().startDate}
                        onInput={(event) => updateTerm("startDate", event.currentTarget.value)}
                      />
                    </label>
                    <label class="curriculum-adoption-field">
                      End date
                      <input
                        type="date"
                        value={targetTerm().endDate}
                        onInput={(event) => updateTerm("endDate", event.currentTarget.value)}
                      />
                    </label>
                    <label class="curriculum-adoption-field">
                      Time zone
                      <input
                        value={targetTerm().timeZone}
                        onInput={(event) => updateTerm("timeZone", event.currentTarget.value)}
                      />
                    </label>
                  </div>
                </fieldset>
              </Show>

              <Show when={curriculumAdoptionOperationNeedsTitle(operation())}>
                <label class="curriculum-adoption-field">
                  New course title
                  <input
                    value={title()}
                    onInput={(event) => {
                      setTitle(event.currentTarget.value);
                      setPreview(undefined);
                      setNotice({
                        kind: "status",
                        text: "New course title updated. Prepare a fresh server-owned proposal when ready.",
                      });
                    }}
                  />
                </label>
              </Show>
              <div class="curriculum-adoption-actions">
                <span>
                  {operationPresentation().requiresSource && !canPreview()
                    ? "Select a compatible source to continue."
                    : "The server will prepare an answer-free proposal before any live record changes."}
                </span>
                <button
                  class="primary-action curriculum-adoption-primary"
                  type="submit"
                  disabled={!canPreview()}
                >
                  {operation() === "imports" ? "Inspect imports" : "Prepare proposal"}
                </button>
              </div>
            </form>
            <Show when={stage() === "recovery"}>
              <RecoveryPanel
                preview={preview()}
                replacements={replacements()}
                onChooseReplacement={(action, question) => {
                  setReplacements((current) => replacePin(current, action, question));
                  setNotice({
                    kind: "status",
                    text: "Replacement selected. Regenerate the proposal to ask the live server to validate it.",
                  });
                }}
                onRegenerate={() => void preparePreview()}
              />
            </Show>
          </section>
        </Match>
        <Match when={stage() === "previewing"}>
          <section class="curriculum-adoption-preview">
            <p role="status">Preparing the live proposal…</p>
          </section>
        </Match>
        <Match when={stage() === "preview"}>
          <Show
            when={operation() === "imports" && preview() === undefined}
            fallback={
              <PreviewPanel
                preview={preview()}
                onBack={() => {
                  setStage("choose");
                  announce("choose");
                }}
                onApply={() => void applyPreview()}
                onSourceDerived={() => void prepareSourceDerived()}
              />
            }
          >
            <ImportInspection
              inspection={importInspection()}
              onBack={() => {
                setStage("choose");
                announce("choose");
              }}
              onFastForward={(assignment) => void prepareFastForward(assignment)}
            />
          </Show>
        </Match>
        <Match when={stage() === "applying"}>
          <section class="curriculum-adoption-preview">
            <p role="status">Applying the live proposal…</p>
          </section>
        </Match>
        <Match when={stage() === "receipt"}>
          <Show when={completed()}>
            {(completedResult) => (
              <ReceiptPanel
                courseReference={completedResult().course}
                receipt={completedResult().receipt}
                reconciliation={reconciliation()}
                onInspect={() => void reconcileReceipt()}
              />
            )}
          </Show>
        </Match>
      </Switch>
    </main>
  );
}

interface PreviewPanelProps {
  readonly preview: Preview | undefined;
  readonly onBack: () => void;
  readonly onApply: () => void;
  readonly onSourceDerived: () => void;
}

function PreviewPanel(props: PreviewPanelProps): JSX.Element {
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

interface RecoveryPanelProps {
  readonly preview: Preview | undefined;
  readonly replacements: ReadonlyArray<CurriculumPinReplacement>;
  readonly onChooseReplacement: (action: UnavailablePinRecoveryAction, question: string) => void;
  readonly onRegenerate: () => void;
}

function RecoveryPanel(props: RecoveryPanelProps): JSX.Element {
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

function ImportInspection(props: {
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
