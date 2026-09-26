// Lazy, read-only inspection of exact saved Blueprint content and recorded metadata.
import { For, Show, createMemo, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintHistoryEntryView } from "../../../generated/api/BlueprintHistoryEntryView";
import type { BlueprintRevisionView } from "../../../generated/api/BlueprintRevisionView";
import type { BlueprintAssessmentContentView } from "../../../generated/api/BlueprintAssessmentContentView";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { ApiRequestError } from "../../api/http_client";
import {
  assessmentDurationDefaultDescription,
  assessmentDurationDisplay,
} from "../../assessment_duration";
import { RecordList, type RecordContent } from "../../components/record_list/record_list";
import {
  RecordOutlineItem,
  RecordOutlineList,
} from "../../components/record_list/record_outline_list";
import { RecordSequence } from "../../components/record_list/record_sequence";
import type { BlueprintAssessmentEntryView } from "../../../generated/api/BlueprintAssessmentEntryView";

interface HistoryProps {
  readonly client: BlueprintCourseClient;
  readonly view: BlueprintCourseView;
  readonly formatDateTime: (timestamp: number | Date) => string;
}

function historyError(error: unknown): string {
  // ASVS 16.5.1: unexpected failures never expose server details or raw error messages.
  if (error instanceof ApiRequestError) {
    if (error.status === 401)
      return "Your session ended. Sign in again to inspect Blueprint history.";
    if (error.status === 403 || error.status === 404)
      return "Blueprint history is unavailable for your Account.";
  }
  return "Blueprint history could not load. Try again.";
}

function policyLabel(name: string): string {
  const words = name.replace(/([a-z])([A-Z])/gu, "$1 $2").replace(/_/gu, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function historicalAssessmentQuestionCount(content: BlueprintAssessmentContentView): number {
  return content.entries.reduce(
    (count, entry) => count + (entry.kind === "fixed" ? 1 : entry.selection_count),
    0,
  );
}

function historicalAssessmentDefaultDuration(content: BlueprintAssessmentContentView): string {
  const seconds = content.defaults.assessment_attempt_time_limit_seconds;
  if (seconds === null)
    return assessmentDurationDefaultDescription(historicalAssessmentQuestionCount(content));
  return assessmentDurationDisplay(seconds);
}

function historicalEntryId(entry: BlueprintAssessmentEntryView): string {
  if (entry.kind === "fixed") {
    const tuple = entry.question.published_question_revision_tuple;
    return `question-${tuple.publishedQuestionId}-${tuple.revisionNumber}`;
  }
  return `pool-${entry.question_pool_id}-${entry.question_pool_edit_number}`;
}

function historyEntryId(entry: BlueprintHistoryEntryView): string {
  return entry.kind === "savedRevision"
    ? `revision-${entry.revisionNumber}`
    : `metadata-${entry.recordedAt}`;
}

function historicalEntryContent(entry: BlueprintAssessmentEntryView): RecordContent {
  const title =
    entry.kind === "fixed"
      ? `Fixed Question ${entry.question.published_question_revision_tuple.publishedQuestionId}, Revision ${entry.question.published_question_revision_tuple.revisionNumber}; ${entry.points_possible} points`
      : `Question Pool ${entry.question_pool_id}, Edit ${entry.question_pool_edit_number}; select ${entry.selection_count}; ${entry.points_per_item} points per Question`;
  const timeLimit =
    entry.question_attempt_time_limit.kind === "limited"
      ? `${assessmentDurationDisplay(entry.question_attempt_time_limit.seconds)} (${entry.question_attempt_time_limit.graceSeconds === 0 ? "no grace" : `${assessmentDurationDisplay(entry.question_attempt_time_limit.graceSeconds)} grace`})`
      : "unlimited";
  const selectedQuestionOrder =
    entry.kind === "pool"
      ? `; selected Question order ${entry.selection_rule.selectedQuestionOrder}`
      : "";

  return {
    title,
    details: [
      {
        kind: "text",
        value: `Scoring ${entry.scoring_rule}; Question Attempt limit ${entry.question_attempt_limit.maxAttempts ?? "unlimited"}; time limit ${timeLimit}${selectedQuestionOrder}`,
      },
    ],
    actions: [],
  };
}

/** Opening history never grants editing or changes the ordinary latest-Revision workspace. */
export function BlueprintHistory(props: HistoryProps): JSX.Element {
  const identity = createMemo(
    () =>
      `${props.view.id}:${props.view.current_revision_tuple.revisionNumber}:${props.view.blueprint_edit_number}`,
  );
  return (
    <Show when={identity()} keyed>
      {(_identity) => (
        <HistoryPanel
          client={props.client}
          view={props.view}
          formatDateTime={props.formatDateTime}
        />
      )}
    </Show>
  );
}

function HistoryPanel(props: HistoryProps): JSX.Element {
  const [revisionOpen, setRevisionOpen] = createSignal(false);
  const [metadataOpen, setMetadataOpen] = createSignal(false);
  const [selectedRevision, setSelectedRevision] = createSignal<string>();
  const [revision, setRevision] = createSignal<BlueprintRevisionView>();
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string>();
  let request = 0;
  onCleanup(() => request++);

  async function inspect(number: string): Promise<void> {
    const currentRequest = ++request;
    setSelectedRevision(number);
    setRevision(undefined);
    setError(undefined);
    setBusy(true);
    try {
      const result = await props.client.getBlueprintRevision(props.view.id, number);
      if (currentRequest !== request) return;
      if (
        result.blueprintRevisionTuple.blueprintCourseId !== props.view.id ||
        result.blueprintRevisionTuple.revisionNumber !== number
      )
        throw new Error("Unexpected Blueprint Revision");
      setRevision(result);
    } catch (failure: unknown) {
      if (currentRequest === request) setError(historyError(failure));
    } finally {
      if (currentRequest === request) setBusy(false);
    }
  }

  function closeInspection(): void {
    request++;
    setSelectedRevision(undefined);
    setRevision(undefined);
    setError(undefined);
    setBusy(false);
  }

  return (
    <section class="blueprint-course-inspection" aria-label="Blueprint history">
      <h2>Blueprint history</h2>
      <p>
        Latest saved content: Revision {props.view.current_revision_tuple.revisionNumber}. History
        is read-only; inspecting it does not replace your current content or local edits.
      </p>
      <details onToggle={(event) => setRevisionOpen(event.currentTarget.open)}>
        <summary>Revision history</summary>
        <Show when={revisionOpen()}>
          <HistoryPage
            client={props.client}
            blueprintCourseId={props.view.id}
            kind="revisions"
            currentRevisionNumber={props.view.current_revision_tuple.revisionNumber}
            formatDateTime={props.formatDateTime}
            onInspect={(number) => void inspect(number)}
          />
        </Show>
      </details>
      <details onToggle={(event) => setMetadataOpen(event.currentTarget.open)}>
        <summary>Recorded metadata changes</summary>
        <Show when={metadataOpen()}>
          <p>Recorded names and availability are separate from saved content Revisions.</p>
          <HistoryPage
            client={props.client}
            blueprintCourseId={props.view.id}
            kind="metadata"
            currentRevisionNumber={props.view.current_revision_tuple.revisionNumber}
            formatDateTime={props.formatDateTime}
            onInspect={(number) => void inspect(number)}
          />
        </Show>
      </details>
      <Show when={selectedRevision()}>
        {(number) => (
          <section
            class="blueprint-course-content-card"
            aria-label={`Read-only Blueprint Revision ${number()}`}
          >
            <h3>
              {number() === props.view.current_revision_tuple.revisionNumber
                ? "Latest saved"
                : "Historical"}{" "}
              Revision {number()} - read-only
            </h3>
            <button type="button" class="quiet-action" onClick={closeInspection}>
              Close Revision inspection
            </button>
            <Show when={busy()}>
              <p role="status">Loading exact saved Revision {number()}.</p>
            </Show>
            <Show when={error()}>
              {(message) => (
                <>
                  <p role="alert">{message()}</p>
                  <button type="button" onClick={() => void inspect(number())}>
                    Retry loading Revision {number()}
                  </button>
                </>
              )}
            </Show>
            <Show when={revision()} keyed>
              {(saved) => <RevisionContent revision={saved} />}
            </Show>
          </section>
        )}
      </Show>
    </section>
  );
}

interface HistoryPageProps {
  readonly client: BlueprintCourseClient;
  readonly blueprintCourseId: string;
  readonly kind: "revisions" | "metadata";
  readonly currentRevisionNumber: string;
  readonly formatDateTime: (timestamp: number | Date) => string;
  readonly onInspect: (revisionNumber: string) => void;
}

/** One bounded page at a time, with a retry tied to the exact failed continuation. */
function HistoryPage(props: HistoryPageProps): JSX.Element {
  const [items, setItems] = createSignal<BlueprintHistoryEntryView[]>([]);
  const [nextCursor, setNextCursor] = createSignal<string | null>(null);
  const [pageCursor, setPageCursor] = createSignal<string>();
  const [busy, setBusy] = createSignal(false);
  const [ready, setReady] = createSignal(false);
  const [error, setError] = createSignal<string>();
  let request = 0;
  onCleanup(() => request++);
  async function load(cursor?: string): Promise<void> {
    const currentRequest = ++request;
    setPageCursor(cursor);
    setBusy(true);
    setError(undefined);
    setReady(false);
    setItems([]);
    setNextCursor(null);
    try {
      const page = await props.client.listBlueprintHistory(
        props.blueprintCourseId,
        props.kind,
        cursor,
        50,
      );
      if (currentRequest !== request) return;
      setItems(page.items);
      setNextCursor(page.nextCursor);
      setReady(true);
    } catch (failure: unknown) {
      if (currentRequest === request) setError(historyError(failure));
    } finally {
      if (currentRequest === request) setBusy(false);
    }
  }
  onMount(() => void load());
  function historyEntryContent(entry: BlueprintHistoryEntryView): RecordContent {
    if (entry.kind === "savedRevision") {
      return {
        title: `Revision ${entry.revisionNumber}${entry.revisionNumber === props.currentRevisionNumber ? " (latest saved)" : " (historical)"}`,
        details: [{ kind: "text", value: `Saved ${props.formatDateTime(entry.savedAt)}` }],
        actions: [
          {
            id: "inspect-revision",
            kind: "command",
            label: `Inspect Revision ${entry.revisionNumber}`,
            onClick: () => props.onInspect(entry.revisionNumber),
          },
        ],
      };
    }

    return {
      title: `${entry.longName} (${entry.shortName}) - ${entry.availability}`,
      details: [
        { kind: "text", value: `Recorded ${props.formatDateTime(entry.recordedAt)}` },
        { kind: "courseClassification", value: entry.classification },
      ],
      actions: [],
    };
  }
  return (
    <section
      aria-label={props.kind === "revisions" ? "Saved Revision page" : "Metadata change page"}
      aria-busy={busy()}
    >
      <Show when={busy()}>
        <p role="status">
          Loading {props.kind === "revisions" ? "Revision history" : "recorded metadata changes"}.
        </p>
      </Show>
      <Show when={error()}>
        {(message) => (
          <>
            <p role="alert">{message()}</p>
            <button type="button" onClick={() => void load(pageCursor())}>
              Retry history page
            </button>
          </>
        )}
      </Show>
      <Show when={ready()}>
        <RecordList
          rows={items()}
          content={historyEntryContent}
          recordId={historyEntryId}
          state={{ kind: "ready" }}
          ariaLabel={props.kind === "revisions" ? "Saved Revisions" : "Recorded metadata changes"}
          emptyState={{
            title: `No ${props.kind === "revisions" ? "saved Revisions" : "recorded metadata changes"}`,
            message: "There are no records on this history page.",
          }}
        />
      </Show>
      <Show when={nextCursor()}>
        {(cursor) => (
          <button type="button" disabled={busy()} onClick={() => void load(cursor())}>
            Next history page
          </button>
        )}
      </Show>
      <Show when={pageCursor() !== undefined}>
        <button type="button" class="quiet-action" disabled={busy()} onClick={() => void load()}>
          Return to newest history page
        </button>
      </Show>
    </section>
  );
}

function RevisionContent(props: { readonly revision: BlueprintRevisionView }): JSX.Element {
  const [selectedAssessment, setSelectedAssessment] =
    createSignal<BlueprintAssessmentContentView>();
  return (
    <>
      <Show
        when={selectedAssessment() === undefined}
        fallback={
          <button
            type="button"
            class="quiet-action"
            onClick={() => setSelectedAssessment(undefined)}
          >
            Return to historical assessment list
          </button>
        }
      >
        <RecordOutlineList
          state={{ kind: "ready" }}
          isEmpty={props.revision.modules.length === 0}
          ariaLabel="Historical Blueprint Modules and Assessments"
          emptyState={{ title: "No Modules", message: "This saved Revision has no Modules." }}
        >
          <For each={props.revision.modules}>
            {(module) => (
              <RecordOutlineItem recordId={module.blueprint_module_id}>
                <section class="blueprint-course-module">
                  <h4>{module.label}</h4>
                  <RecordOutlineList
                    state={{ kind: "ready" }}
                    isEmpty={module.assessments.length === 0}
                    ariaLabel={`${module.label} historical Assessments`}
                    emptyState={{
                      title: "No Assessments",
                      message: "This Module has no saved Assessments.",
                    }}
                  >
                    <For each={module.assessments}>
                      {(assessment) => (
                        <RecordOutlineItem recordId={assessment.blueprint_assessment_id}>
                          <span>{assessment.content.title}</span>
                          <button
                            type="button"
                            class="quiet-action"
                            onClick={() => setSelectedAssessment(assessment.content)}
                          >
                            View historical assessment
                          </button>
                        </RecordOutlineItem>
                      )}
                    </For>
                  </RecordOutlineList>
                </section>
              </RecordOutlineItem>
            )}
          </For>
        </RecordOutlineList>
      </Show>
      <Show when={selectedAssessment()}>
        {(content) => (
          <section aria-label="Read-only historical assessment">
            <h4>{content().title}</h4>
            <p>Assessment type: {content().assessment_type}</p>
            {/* ASVS 1.2.1: authored instructions remain escaped plain text, never HTML. */}
            <p style={{ "white-space": "pre-wrap" }}>{content().instructions}</p>
            <h5>Question entries</h5>
            <RecordSequence
              rows={content().entries}
              content={historicalEntryContent}
              recordId={historicalEntryId}
              state={{ kind: "ready" }}
              ariaLabel="Historical Question and Pool entries"
              emptyState={{
                title: "No Question entries",
                message: "This historical Assessment has no reusable entries.",
              }}
            />
            <h5>Assessment defaults</h5>
            <dl>
              <dt>Assessment Attempt time limit</dt>
              <dd>{historicalAssessmentDefaultDuration(content())}</dd>
              <dt>Assessment Attempt limit</dt>
              <dd>{content().defaults.assessment_attempt_limit ?? "Unlimited"}</dd>
              <dt>Late work rule</dt>
              <dd>{content().defaults.late_work_rule}</dd>
              <dt>Student feedback release rule</dt>
              <dd>
                <dl>
                  <For each={Object.entries(content().defaults.student_feedback_release_rule)}>
                    {([name, value]) => (
                      <>
                        <dt>{policyLabel(name)}</dt>
                        <dd>{value}</dd>
                      </>
                    )}
                  </For>
                </dl>
              </dd>
              <For each={Object.entries(content().defaults.activity_rules)}>
                {([name, value]) => (
                  <>
                    <dt>{policyLabel(name)}</dt>
                    <dd>{value}</dd>
                  </>
                )}
              </For>
            </dl>
          </section>
        )}
      </Show>
    </>
  );
}
