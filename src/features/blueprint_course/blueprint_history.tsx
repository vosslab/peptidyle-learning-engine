// Lazy, read-only inspection of exact saved Blueprint content and recorded metadata.
import { For, Show, createMemo, createSignal, onCleanup, onMount, type JSX } from "solid-js";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintHistoryEntryView } from "../../../generated/api/BlueprintHistoryEntryView";
import type { BlueprintRevisionView } from "../../../generated/api/BlueprintRevisionView";
import type { BlueprintAssessmentContentView } from "../../../generated/api/BlueprintAssessmentContentView";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import { ApiRequestError } from "../../api/http_client";

interface HistoryProps {
  readonly client: BlueprintCourseClient;
  readonly view: BlueprintCourseView;
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

function recordedTime(timestamp: number): string {
  const date = new Date(timestamp);
  return Number.isNaN(date.getTime()) ? `Unix milliseconds ${timestamp}` : date.toLocaleString();
}

function policyLabel(name: string): string {
  const words = name.replace(/([a-z])([A-Z])/gu, "$1 $2").replace(/_/gu, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** Opening history never grants editing or changes the ordinary latest-Revision workspace. */
export function BlueprintHistory(props: HistoryProps): JSX.Element {
  const identity = createMemo(
    () =>
      `${props.view.reference}:${props.view.current_revision.revision}:${props.view.metadata_etag}`,
  );
  return (
    <Show when={identity()} keyed>
      {(_identity) => <HistoryPanel client={props.client} view={props.view} />}
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
      const result = await props.client.getBlueprintRevision(props.view.reference, number);
      if (currentRequest !== request) return;
      if (
        result.blueprintRevision.reference !== props.view.reference ||
        result.blueprintRevision.revision !== number
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
        Latest saved content: Revision {props.view.current_revision.revision}. History is read-only;
        inspecting it does not replace your current content or local edits.
      </p>
      <details onToggle={(event) => setRevisionOpen(event.currentTarget.open)}>
        <summary>Revision history</summary>
        <Show when={revisionOpen()}>
          <HistoryPage
            client={props.client}
            reference={props.view.reference}
            kind="revisions"
            currentRevision={props.view.current_revision.revision}
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
            reference={props.view.reference}
            kind="metadata"
            currentRevision={props.view.current_revision.revision}
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
              {number() === props.view.current_revision.revision ? "Latest saved" : "Historical"}{" "}
              Revision {number()} — read-only
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
  readonly reference: string;
  readonly kind: "revisions" | "metadata";
  readonly currentRevision: string;
  readonly onInspect: (revision: string) => void;
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
      const page = await props.client.listBlueprintHistory(props.reference, props.kind, cursor, 50);
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
      <Show when={ready() && items().length === 0}>
        <p>
          No {props.kind === "revisions" ? "saved Revisions" : "recorded metadata changes"} on this
          page.
        </p>
      </Show>
      <ul class="blueprint-course-assessment-list">
        <For each={items()}>
          {(item) => (
            <li>
              <Show when={item.kind === "savedRevision" ? item : undefined}>
                {(saved) => (
                  <>
                    <span>
                      Revision {saved().revision}
                      {saved().revision === props.currentRevision
                        ? " (latest saved)"
                        : " (historical)"}{" "}
                      — saved {recordedTime(saved().savedAt)}
                    </span>
                    <button
                      type="button"
                      class="quiet-action"
                      onClick={() => props.onInspect(saved().revision)}
                    >
                      Inspect Revision {saved().revision}
                    </button>
                  </>
                )}
              </Show>
              <Show when={item.kind === "metadataChange" ? item : undefined}>
                {(metadata) => (
                  <span>
                    {metadata().longName} ({metadata().shortName}) — {metadata().availability};
                    recorded {recordedTime(metadata().recordedAt)}
                  </span>
                )}
              </Show>
            </li>
          )}
        </For>
      </ul>
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
        <For each={props.revision.modules}>
          {(module) => (
            <section class="blueprint-course-module">
              <h4>{module.label}</h4>
              <ul class="blueprint-course-assessment-list">
                <For each={module.assessments}>
                  {(assessment) => (
                    <li>
                      <span>{assessment.content.title}</span>
                      <button
                        type="button"
                        class="quiet-action"
                        onClick={() => setSelectedAssessment(assessment.content)}
                      >
                        View historical assessment
                      </button>
                    </li>
                  )}
                </For>
              </ul>
            </section>
          )}
        </For>
        <Show when={props.revision.modules.length === 0}>
          <p>No Modules in this saved Revision.</p>
        </Show>
      </Show>
      <Show when={selectedAssessment()}>
        {(content) => (
          <section aria-label="Read-only historical assessment">
            <h4>{content().title}</h4>
            <p>Assessment type: {content().assessment_type}</p>
            {/* ASVS 1.2.1: authored instructions remain escaped plain text, never HTML. */}
            <p style={{ "white-space": "pre-wrap" }}>{content().instructions}</p>
            <h5>Question entries</h5>
            <ol>
              <For each={content().entries}>
                {(entry) => (
                  <li>
                    {entry.kind === "fixed"
                      ? `Fixed Question ${entry.question.reference.questionId}, Revision ${entry.question.reference.revisionNumber}; ${entry.points_possible} points`
                      : `Question Pool ${entry.question_pool_revision.questionPoolId}, Revision ${entry.question_pool_revision.revisionNumber}; select ${entry.selection_count}; ${entry.points_per_item} points per Question`}{" "}
                    — scoring {entry.scoring_rule}; Question Attempt limit{" "}
                    {entry.question_attempt_limit.maxAttempts ?? "unlimited"}; time limit{" "}
                    {entry.question_attempt_time_limit.kind === "limited"
                      ? `${entry.question_attempt_time_limit.seconds} seconds (${entry.question_attempt_time_limit.graceSeconds} grace seconds)`
                      : "unlimited"}
                    <Show when={entry.kind === "pool" ? entry : undefined}>
                      {(pool) => (
                        <span>
                          ; selected Question order {pool().selection_rule.selectedQuestionOrder}
                        </span>
                      )}
                    </Show>
                  </li>
                )}
              </For>
            </ol>
            <Show when={content().entries.length === 0}>
              <p>No Question entries.</p>
            </Show>
            <h5>Assessment defaults</h5>
            <dl>
              <dt>Assessment Attempt time limit (seconds)</dt>
              <dd>{content().defaults.assessment_attempt_time_limit_seconds ?? "None"}</dd>
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
