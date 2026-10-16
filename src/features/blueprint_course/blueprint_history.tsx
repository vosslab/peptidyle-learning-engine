// Lazy, read-only inspection of exact saved Blueprint content and recorded metadata.
import { For, Show, createMemo, createSignal, onCleanup, type JSX } from "solid-js";
import type { BlueprintCourseView } from "../../../generated/api/BlueprintCourseView";
import type { BlueprintHistoryEntryView } from "../../../generated/api/BlueprintHistoryEntryView";
import type { BlueprintRevisionView } from "../../../generated/api/BlueprintRevisionView";
import type { BlueprintAssessmentContentView } from "../../../generated/api/BlueprintAssessmentContentView";
import type {
  BlueprintCourseClient,
  BlueprintHistoryPage,
  BlueprintHistoryQuery,
} from "../../api/blueprint_course";
import { ApiRequestError } from "../../api/http_client";
import {
  assessmentDurationDefaultDescription,
  assessmentDurationDisplay,
} from "../../assessment_duration";
import type { RecordContent } from "../../components/record_list/record_list";
import type {
  RecordField,
  RecordFilterChoices,
  RecordListQuery,
} from "../../components/record_list/record_fields";
import { SearchBlock } from "../search/search_block";
import { SearchControls } from "../search/search_controls";
import { createSearchState } from "../search/search_state";
import type { SearchDefinition } from "../search/search_session";
import {
  RecordOutlineItem,
  RecordOutlineList,
} from "../../components/record_list/record_outline_list";
import { RecordSequence } from "../../components/record_list/record_sequence";
import type { BlueprintAssessmentEntryView } from "../../../generated/api/BlueprintAssessmentEntryView";
import type { RecognitionTitleMaps } from "../../api/recognition_titles";
import { labeledQuestionRecognition, recognitionTitle } from "../recognition_label";
import { recognitionTitlesResource } from "../recognition_titles_load";

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
  return `pool-${entry.question_pool_id}`;
}

function historyEntryId(entry: BlueprintHistoryEntryView): string {
  return entry.kind === "savedRevision"
    ? `revision-${entry.revisionNumber}`
    : `metadata-${entry.blueprintEditNumber}`;
}

function historicalEntryContent(
  entry: BlueprintAssessmentEntryView,
  titles: RecognitionTitleMaps,
): RecordContent {
  const timeLimit =
    entry.question_attempt_time_limit.kind === "limited"
      ? `${assessmentDurationDisplay(entry.question_attempt_time_limit.seconds)} (${entry.question_attempt_time_limit.graceSeconds === 0 ? "no grace" : `${assessmentDurationDisplay(entry.question_attempt_time_limit.graceSeconds)} grace`})`
      : "unlimited";
  const scoring = `Scoring ${entry.scoring_rule}; Question Attempt limit ${entry.question_attempt_limit.maxAttempts ?? "unlimited"}; time limit ${timeLimit}`;
  if (entry.kind === "fixed") {
    const tuple = entry.question.published_question_revision_tuple;
    const line = labeledQuestionRecognition(
      titles.questions.get(tuple.publishedQuestionId),
      tuple.publishedQuestionId,
      tuple.revisionNumber,
    );
    return {
      title: line.title,
      details: [
        { kind: "text", label: "Question ID", value: tuple.publishedQuestionId },
        { kind: "text", label: "Revision", value: String(tuple.revisionNumber) },
        { kind: "text", label: "Points", value: String(entry.points_possible) },
        { kind: "text", value: scoring },
      ],
      actions: [],
    };
  }
  return {
    title: recognitionTitle(titles.pools.get(entry.question_pool_id), "Question Pool"),
    details: [
      { kind: "text", label: "Question Pool ID", value: entry.question_pool_id },
      { kind: "text", label: "Questions selected", value: String(entry.selection_count) },
      { kind: "text", label: "Points per Question", value: String(entry.points_per_item) },
      {
        kind: "text",
        value: scoring,
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
        result.blueprintCourseRevisionTuple.blueprintCourseId !== props.view.id ||
        result.blueprintCourseRevisionTuple.revisionNumber !== number
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
              {(saved) => <RevisionContent client={props.client} revision={saved} />}
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

type HistorySortKey = "sequence" | "recordedAt" | "name";
type HistoryFilterKey = "availability";

type HistoryRow = BlueprintHistoryEntryView;

function historyFields(
  kind: "revisions" | "metadata",
  formatDateTime: (timestamp: number | Date) => string,
): ReadonlyArray<RecordField<HistoryRow, HistorySortKey, HistoryFilterKey>> {
  if (kind === "revisions")
    return [
      {
        id: "revision",
        label: "Revision",
        role: "identity",
        sortKey: "sequence",
        value: (row) => (row.kind === "savedRevision" ? `Revision ${row.revisionNumber}` : ""),
        defaultColumn: { priority: 0, width: "12rem" },
      },
      {
        id: "savedAt",
        label: "Saved",
        sortKey: "recordedAt",
        value: (row) => (row.kind === "savedRevision" ? row.savedAt : null),
        render: (row) =>
          row.kind === "savedRevision" ? (
            <time dateTime={new Date(row.savedAt).toISOString()}>
              {formatDateTime(row.savedAt)}
            </time>
          ) : null,
        defaultColumn: { priority: 1, width: "15rem" },
      },
    ];
  return [
    {
      id: "name",
      label: "Long name",
      role: "identity",
      sortKey: "name",
      value: (row) => (row.kind === "metadataChange" ? row.longName : ""),
      defaultColumn: { priority: 0, width: "18rem" },
    },
    {
      id: "shortName",
      label: "Short name",
      value: (row) => (row.kind === "metadataChange" ? row.shortName : null),
      defaultColumn: { priority: 1, width: "12rem" },
    },
    {
      id: "availability",
      label: "Availability",
      filterKey: "availability",
      value: (row) => (row.kind === "metadataChange" ? row.availability : null),
      defaultColumn: { priority: 2, width: "10rem" },
    },
    {
      id: "recordedAt",
      label: "Recorded",
      sortKey: "recordedAt",
      value: (row) => (row.kind === "metadataChange" ? row.recordedAt : null),
      render: (row) =>
        row.kind === "metadataChange" ? (
          <time dateTime={new Date(row.recordedAt).toISOString()}>
            {formatDateTime(row.recordedAt)}
          </time>
        ) : null,
      defaultColumn: { priority: 3, width: "15rem" },
    },
    {
      id: "theme",
      label: "Theme",
      value: (row) => (row.kind === "metadataChange" ? row.theme : null),
      defaultColumn: { priority: 4, width: "10rem", visible: false },
    },
  ];
}

/** Each independent history stream owns a complete-result query and stable keyset. */
function HistoryPage(props: HistoryPageProps): JSX.Element {
  const initialQuery: BlueprintHistoryQuery = {
    q: "",
    sort: "sequence",
    direction: "descending",
    availability: [],
  };
  const content = (entry: HistoryRow): RecordContent => {
    if (entry.kind === "savedRevision")
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
    return {
      title: `${entry.longName} (${entry.shortName}) - ${entry.availability}`,
      details: [
        { kind: "text", value: `Recorded ${props.formatDateTime(entry.recordedAt)}` },
        { kind: "text", value: `Theme: ${entry.theme}` },
        { kind: "courseClassification", value: entry.classification },
      ],
      actions: [],
    };
  };
  const definition: SearchDefinition<BlueprintHistoryQuery, HistoryRow, BlueprintHistoryPage> = {
    initialQuery,
    maximumPageSize: 100,
    cleanup: (query) => ({
      q: query.q.trim().toLowerCase().slice(0, 120),
      sort: props.kind === "revisions" && query.sort === "name" ? "sequence" : query.sort,
      direction: query.direction,
      availability: props.kind === "metadata" ? [...new Set(query.availability)].sort() : [],
    }),
    isEligible: () => true,
    getText: (query) => query.q,
    setText: (query, q) => ({ ...query, q }),
    rowId: historyEntryId,
    content,
    fetchPage: async (query, cursor, size) => {
      const page = await props.client.listBlueprintHistory(
        props.blueprintCourseId,
        props.kind,
        query,
        cursor,
        size,
      );
      return { items: page.items, nextCursor: page.nextCursor, filterCounts: page };
    },
    appliedFilters: (query) =>
      query.availability.map((value) => ({
        id: value,
        label: `Availability: ${value}`,
        clear: (current: BlueprintHistoryQuery): BlueprintHistoryQuery => ({
          ...current,
          availability: current.availability.filter((item) => item !== value),
        }),
      })),
  };
  const state = createSearchState(definition, { runOnMount: true });
  const recordQuery: RecordListQuery<HistorySortKey, HistoryFilterKey> = {
    kind: "paged",
    sort: {
      get value() {
        return { key: state.query().sort, direction: state.query().direction };
      },
      onChange: (sort) =>
        void state.apply((current) => ({ ...current, sort: sort.key, direction: sort.direction })),
    },
    ...(props.kind === "metadata"
      ? {
          filters: {
            get values(): ReadonlyMap<HistoryFilterKey, ReadonlyArray<string>> {
              return new Map<HistoryFilterKey, ReadonlyArray<string>>([
                ["availability", state.query().availability],
              ]);
            },
            onChange: (_key: HistoryFilterKey, values: ReadonlyArray<string>) =>
              void state.apply((current) => ({
                ...current,
                availability: values.filter(
                  (value): value is "public" | "archived" =>
                    value === "public" || value === "archived",
                ),
              })),
            onClear: () => void state.apply((current) => ({ ...current, availability: [] })),
            choices: async (
              _key: HistoryFilterKey,
              search: string,
            ): Promise<RecordFilterChoices> => {
              const query = state.query();
              const page = await props.client.listBlueprintHistory(
                props.blueprintCourseId,
                props.kind,
                { ...query, availability: [] },
                null,
                1,
              );
              const choices = (["public", "archived"] as const).map((key) => ({
                key,
                label: key === "public" ? "Public" : "Archived",
                count:
                  key === "public"
                    ? page.availabilityFacets.publicCount
                    : page.availabilityFacets.archivedCount,
              }));
              const term = search.trim().toLowerCase();
              return {
                choices: choices.filter((choice) => choice.label.toLowerCase().includes(term)),
                selected: choices.filter((choice) => query.availability.includes(choice.key)),
                nextCursor: null,
              };
            },
          },
        }
      : {}),
  };
  return (
    <SearchBlock
      layout="above"
      state={state}
      controls={
        <SearchControls
          state={state}
          textLabel={props.kind === "revisions" ? "Search Revisions" : "Search metadata changes"}
          textPlaceholder={props.kind === "revisions" ? "Revision number" : "Name or theme"}
        />
      }
      collection={{
        ariaLabel: props.kind === "revisions" ? "Saved Revisions" : "Recorded metadata changes",
        emptyState: {
          title:
            props.kind === "revisions" ? "No matching Revisions" : "No matching metadata changes",
          message: "Try another search or filter.",
        },
        fields: historyFields(props.kind, props.formatDateTime),
        query: recordQuery,
      }}
    />
  );
}

function RevisionContent(props: {
  readonly client: BlueprintCourseClient;
  readonly revision: BlueprintRevisionView;
}): JSX.Element {
  const [selectedAssessment, setSelectedAssessment] =
    createSignal<BlueprintAssessmentContentView>();
  const recognition = recognitionTitlesResource(
    () => props.client,
    () => {
      const content = selectedAssessment();
      if (content === undefined) return { questionIds: [], poolIds: [] };
      const questionIds: string[] = [];
      const poolIds: string[] = [];
      for (const entry of content.entries) {
        if (entry.kind === "fixed") {
          questionIds.push(entry.question.published_question_revision_tuple.publishedQuestionId);
        } else {
          poolIds.push(entry.question_pool_id);
        }
      }
      return { questionIds, poolIds };
    },
  );
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
            <Show when={recognition.status()}>
              <p role="status">{recognition.status()}</p>
            </Show>
            <RecordSequence
              rows={content().entries}
              content={(entry) => historicalEntryContent(entry, recognition.titles())}
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
