import { createEffect, createSignal, For, onCleanup, Show, type JSX } from "solid-js";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { BlueprintCourseClient } from "../api/blueprint_course";
import type {
  CourseStudentWorkRecoveryClient,
  RecoveredAttempt,
  RecoverySelection,
  RecoverySummary,
} from "../api/course_student_work_recovery";
import { labeledQuestionRecognition } from "../features/recognition_label";
import { recognitionTitlesResource } from "../features/recognition_titles_load";
import { ApiRequestError } from "../api/http_client/error";
import { createDisplayDateTimeFormatter } from "../format_datetime";
import type { RecordContent, RecordFact } from "./data_sheet/record_content";
import type { RecordField, RecordListQuery, RecordSort } from "./data_sheet/record_fields";
import { SearchBlock } from "./search_block/search_block";
import { SearchControls } from "./search_block/search_controls";
import { createLocalCompleteSearch } from "./search_block/local_complete_search_state";
import {
  compareRecoveredQuestionRecords,
  recoveredQuestionRecordContent,
  recoveredQuestionRecordFields,
  recoveredQuestionRecordKeyword,
  type RecoveredQuestionRecord,
} from "./recovered_question_record_fields";
import { createSearchState } from "./search_block/search_state";
import type { SearchPageRequest } from "./search_block/search_session";
import { formatAssessmentActivity } from "./student_assessment_presentation";

function formatRecoveryInstant(
  value: string | null,
  formatDateTime: ReturnType<typeof createDisplayDateTimeFormatter>,
  unsetLabel: string,
): string {
  if (value === null) return unsetLabel;
  const timestamp = Date.parse(value);
  return Number.isNaN(timestamp)
    ? "Retained time unavailable"
    : formatAssessmentActivity(timestamp, formatDateTime);
}

function recoveryInstantFact(
  label: string,
  value: string | null,
  formatDateTime: ReturnType<typeof createDisplayDateTimeFormatter>,
  unsetLabel: string,
): RecordFact {
  if (value === null) return { kind: "text", label, value: unsetLabel };
  const timestamp = Date.parse(value);
  if (Number.isNaN(timestamp)) return { kind: "text", label, value: "Retained time unavailable" };
  return {
    kind: "time",
    label,
    dateTime: value,
    value: formatAssessmentActivity(timestamp, formatDateTime),
  };
}

function Evidence(props: { readonly label: string; readonly text: string | null }): JSX.Element {
  // ASVS 1.2.1: Solid text interpolation only, including retained backend HTML.
  return (
    <details>
      <summary>{props.label}</summary>
      <Show when={props.text !== null} fallback={<p>No retained evidence.</p>}>
        <pre
          style={{
            "white-space": "pre-wrap",
            "overflow-wrap": "anywhere",
            "max-height": "24rem",
            overflow: "auto",
          }}
        >
          {props.text}
        </pre>
      </Show>
    </details>
  );
}

/** One selected Attempt's retained evidence; the recovery radio remains a separate operation. */
function RecoveredQuestions(props: {
  readonly attempt: RecoveredAttempt;
  readonly questionTitles: ReadonlyMap<string, string>;
}): JSX.Element {
  // ASVS 14.3.3: retain authorized evidence only in this short-lived component's memory.
  const questions = (): ReadonlyArray<RecoveredQuestionRecord> =>
    props.attempt.questions.map((question) => ({
      ...question,
      questionTitle: labeledQuestionRecognition(
        props.questionTitles.get(question.publishedQuestionRevisionTuple.publishedQuestionId),
        question.publishedQuestionRevisionTuple.publishedQuestionId,
        question.publishedQuestionRevisionTuple.revisionNumber,
      ).title,
    }));
  const questionSearch = createLocalCompleteSearch(
    { rows: questions },
    {
      fields: recoveredQuestionRecordFields,
      rowId: (question) => question.issuedPosition.toString(),
      keyword: recoveredQuestionRecordKeyword,
      compare: compareRecoveredQuestionRecords,
      content: recoveredQuestionRecordContent,
    },
  );
  return (
    <SearchBlock
      layout="above"
      controls={
        <SearchControls
          state={questionSearch.state}
          textLabel="Search recovered Question evidence"
        />
      }
      state={questionSearch.state}
      collection={{
        ariaLabel: "Recovered Question evidence",
        initialMode: "cards",
        fields: recoveredQuestionRecordFields,
        query: questionSearch.query,
        emptyState: { title: "No recovered Question evidence matches your search." },
        renderBody: (question) => (
          <div>
            <Evidence label="Delivery and exact Revision" text={question().deliveryText} />
            <Evidence label="Pool selection" text={question().poolText} />
            <Evidence label="Question Attempt" text={question().attemptText} />
            <Evidence label="Presentation" text={question().presentationText} />
            <Evidence label="Reproduction evidence" text={question().reproductionText} />
            <Evidence
              label="Retained backend document (inert text)"
              text={question().backendDocumentText}
            />
            <Evidence label="Saved response" text={question().savedResponseText} />
            <Evidence label="Finalized response" text={question().finalizedResponseText} />
            <Evidence label="Retained grading outcome" text={question().gradingText} />
            <Show when={question().unavailableEvidence.length > 0}>
              <p>Unavailable evidence:</p>
              <ul>
                <For each={question().unavailableEvidence}>{(item) => <li>{item}</li>}</For>
              </ul>
            </Show>
          </div>
        ),
      }}
    />
  );
}

export function archivedAttemptVisibleFacts(
  attempt: RecoverySummary,
  formatDateTime: ReturnType<typeof createDisplayDateTimeFormatter>,
): { readonly title: string; readonly details: ReadonlyArray<RecordFact> } {
  return {
    title: `${attempt.assessmentTitle}: Attempt ${attempt.assessmentAttemptNumber}`,
    details: [
      {
        kind: "text",
        label: "Roster ID",
        value:
          attempt.courseRosterTuple === null ? "Not retained" : attempt.courseRosterTuple.rosterId,
      },
      recoveryInstantFact("Started", attempt.startedAt, formatDateTime, "No retained start time"),
      recoveryInstantFact(
        "Submitted",
        attempt.submittedAt,
        formatDateTime,
        "No retained submission time",
      ),
      recoveryInstantFact(
        "Deletion cutoff",
        attempt.deleteDueAt,
        formatDateTime,
        "No retained deletion cutoff",
      ),
    ],
  };
}

/** Deliberate evidence access, not an archive browser or restoration command. */
export function CourseStudentWorkRecovery(props: {
  readonly courseInstanceId: CourseInstanceId;
  readonly client: CourseStudentWorkRecoveryClient &
    Pick<BlueprintCourseClient, "loadRecognitionTitles">;
  /** Authenticated Instructor Account preference, never the Course or browser zone. */
  readonly displayTimeZone: string;
}): JSX.Element {
  const formatDateTime = createDisplayDateTimeFormatter(props.displayTimeZone);
  const [opened, setOpened] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [selected, setSelected] = createSignal("");
  const [selectedSummary, setSelectedSummary] = createSignal<RecoverySummary>();
  const [evidence, setEvidence] = createSignal<RecoveredAttempt>();
  const [message, setMessage] = createSignal("");
  const recognition = recognitionTitlesResource(
    () => props.client,
    () => ({
      questionIds: (evidence()?.questions ?? []).map(
        (question) => question.publishedQuestionRevisionTuple.publishedQuestionId,
      ),
      poolIds: [],
    }),
  );
  type RecoveryAttempt = RecoverySelection["attempts"][number];
  type RecoveryQuery = {
    search: string;
    assessmentIds: ReadonlyArray<RecoverySummary["assessmentId"]>;
    sort: "attemptAscending" | "attemptDescending";
  };
  type RecoverySortKey = "attempt";
  type RecoveryFilterKey = "assessment";
  const fields: ReadonlyArray<RecordField<RecoveryAttempt, RecoverySortKey, RecoveryFilterKey>> = [
    {
      id: "attempt",
      label: "Attempt",
      role: "identity",
      value: (attempt) => `${attempt.assessmentTitle}: Attempt ${attempt.assessmentAttemptNumber}`,
      defaultColumn: { priority: 0, width: "18rem" },
    },
    {
      id: "attempt_id",
      label: "Attempt ID",
      sortKey: "attempt",
      value: (attempt) => attempt.assessmentAttemptId,
      defaultColumn: { priority: 1, width: "12rem", visible: false },
    },
    {
      id: "assessment",
      label: "Assessment",
      filterKey: "assessment",
      value: (attempt) => attempt.assessmentId,
      defaultColumn: { priority: 2, width: "12rem" },
    },
    {
      id: "roster",
      label: "Roster ID",
      value: (attempt) => attempt.courseRosterTuple?.rosterId ?? null,
      defaultColumn: { priority: 3, width: "10rem" },
    },
    {
      id: "started",
      label: "Started",
      value: (attempt) => attempt.startedAt,
      render: (attempt) =>
        formatRecoveryInstant(attempt.startedAt, formatDateTime, "No retained start time"),
      defaultColumn: { priority: 4, width: "13rem" },
    },
    {
      id: "submitted",
      label: "Submitted",
      value: (attempt) => attempt.submittedAt,
      render: (attempt) =>
        formatRecoveryInstant(attempt.submittedAt, formatDateTime, "No retained submission time"),
      defaultColumn: { priority: 5, width: "13rem" },
    },
  ];
  const searchState = createSearchState<
    RecoveryQuery,
    RecoveryAttempt,
    RecoverySelection["assessmentChoices"]
  >(
    {
      initialQuery: { search: "", assessmentIds: [], sort: "attemptAscending" },
      maximumPageSize: 100,
      cleanup: (query) => ({ ...query, search: query.search.trim() }),
      isEligible: () => true,
      getText: (query) => query.search,
      setText: (query, text) => ({ ...query, search: text }),
      rowId: (attempt) => attempt.assessmentAttemptId,
      content: attemptContent,
      appliedFilters: (query) =>
        query.assessmentIds.map((assessmentId) => ({
          id: `assessment:${assessmentId}`,
          label: `Assessment: ${
            assessmentChoices().find((choice) => choice.key === assessmentId)?.label ?? assessmentId
          }`,
          clear: (current: RecoveryQuery): RecoveryQuery => ({
            ...current,
            assessmentIds: current.assessmentIds.filter((id) => id !== assessmentId),
          }),
        })),
      fetchPage: async (query, requestPage: SearchPageRequest) => {
        if (requestPage.pageSize !== 50 && requestPage.pageSize !== 100) {
          throw new Error("Recovery supports page sizes 50 and 100");
        }
        const result = await props.client.selectArchivedStudentWork(props.courseInstanceId, {
          ...query,
          pageNumber: requestPage.pageNumber,
          pageSize: requestPage.pageSize,
        });
        if (result.pageSize !== requestPage.pageSize) {
          throw new Error("Recovery returned a different page size than requested");
        }
        return {
          items: result.attempts,
          pageNumber: result.pageNumber,
          pageSize: requestPage.pageSize,
          totalCount: result.totalCount,
          filterCounts: result.assessmentChoices,
        };
      },
    },
    {},
  );
  const assessmentChoices = (): RecoverySelection["assessmentChoices"] =>
    searchState.state().filterCounts ?? [];
  const recordQuery: RecordListQuery<RecoverySortKey, RecoveryFilterKey> = {
    kind: "paged",
    sort: {
      get value(): RecordSort<RecoverySortKey> {
        return {
          key: "attempt",
          direction: searchState.query().sort === "attemptAscending" ? "ascending" : "descending",
        };
      },
      onChange: (value) =>
        void searchState.sort((current) => ({
          ...current,
          sort: value.direction === "ascending" ? "attemptAscending" : "attemptDescending",
        })),
    },
    filters: {
      get values() {
        return new Map<RecoveryFilterKey, ReadonlyArray<string>>([
          ["assessment", searchState.query().assessmentIds],
        ]);
      },
      onChange: (_field, values) =>
        void searchState.apply((current) => ({
          ...current,
          assessmentIds: values,
        })),
      onClear: () => void searchState.apply((current) => ({ ...current, assessmentIds: [] })),
      choices: (_field, choiceSearch, _cursor, signal) =>
        Promise.resolve({
          choices: signal.aborted
            ? []
            : assessmentChoices()
                .filter((choice) => choice.label.toLowerCase().includes(choiceSearch.toLowerCase()))
                .map((choice) => ({ key: choice.key, label: choice.label, count: choice.count })),
          selected: searchState.query().assessmentIds.map((id) => {
            const choice = assessmentChoices().find((entry) => entry.key === id);
            return { key: id, label: choice?.label ?? id, count: choice?.count ?? 0 };
          }),
          nextCursor: null,
        }),
    },
  };
  const selectedAttemptIds = (): ReadonlySet<string> =>
    selected() === "" ? new Set() : new Set([selected()]);
  function attemptContent(attempt: RecoveryAttempt): RecordContent {
    const visible = archivedAttemptVisibleFacts(attempt, formatDateTime);
    return { title: visible.title, details: visible.details, actions: [] };
  }
  let generation = 0;
  let action: HTMLButtonElement | undefined;
  let heading: HTMLHeadingElement | undefined;
  function clear(): void {
    generation += 1;
    setOpened(false);
    setBusy(false);
    setSelected("");
    setSelectedSummary(undefined);
    setEvidence(undefined);
    setMessage("");
  }
  createEffect(() => {
    void props.courseInstanceId;
    clear();
  });
  onCleanup(clear);
  function close(): void {
    clear();
    action?.focus();
  }
  function failure(error: unknown): void {
    // ASVS 16.5.1: do not disclose protocol, SQL or retained evidence in errors.
    setSelected("");
    setSelectedSummary(undefined);
    setEvidence(undefined);
    setMessage(
      error instanceof ApiRequestError && error.status === 404
        ? "Archived Student Work is unavailable through your current Course access or retention period."
        : "Archived Student Work could not be loaded. Try again.",
    );
  }
  async function recover(): Promise<void> {
    const attempt = selected();
    if (busy() || selectedSummary()?.assessmentAttemptId !== attempt) return;
    const token = ++generation;
    setBusy(true);
    setMessage("");
    setEvidence(undefined);
    try {
      const result = await props.client.recoverArchivedStudentWork(props.courseInstanceId, attempt);
      if (token !== generation) return;
      setEvidence(result);
    } catch (error: unknown) {
      if (token === generation) failure(error);
    } finally {
      if (token === generation) setBusy(false);
    }
  }
  return (
    <section aria-label="Archived Student Work recovery">
      <button
        ref={(element) => {
          action = element;
        }}
        type="button"
        class="quiet-button"
        disabled={busy()}
        onClick={() => {
          clear();
          setOpened(true);
          void searchState.open(searchState.definition.initialQuery);
          queueMicrotask(() => heading?.focus());
        }}
      >
        Recover archived Student Work
      </button>
      <Show when={opened()}>
        <section class="card" aria-busy={busy()}>
          <h2
            ref={(element) => {
              heading = element;
            }}
            tabIndex={-1}
          >
            Recover archived Student Work
          </h2>
          <p>
            Access retained evidence for this Course until its original deletion cutoff. Recovery
            does not restore ordinary access or extend retention.
          </p>
          <button type="button" class="quiet-button" onClick={close}>
            Close recovery
          </button>
          <Show when={busy()}>
            <p role="status">Loading archived Student Work...</p>
          </Show>
          <Show when={message() !== ""}>
            <p role="alert">{message()}</p>
          </Show>
          <SearchBlock
            layout="above"
            state={searchState}
            controls={
              <SearchControls
                state={searchState}
                textLabel="Assessment title"
                maxlength={200}
                disabled={busy}
                onSubmit={() => {
                  if (busy()) return;
                  const value = searchState.typedText().trim();
                  if (value.length > 200 || /\p{Cc}/u.test(value)) {
                    setMessage("Enter an Assessment title within 200 characters.");
                    return;
                  }
                  setMessage("");
                  void searchState.submit();
                }}
              />
            }
            collection={{
              ariaLabel: "Retained Assessment Attempts",
              fields,
              query: recordQuery,
              emptyState: { title: "No retained archived Attempts are available." },
              disabled: busy,
              selection: () => ({
                kind: "radio",
                selectedIds: selectedAttemptIds,
                disabled: () => busy(),
                onChange: (attempt, isSelected): void => {
                  if (busy() || !isSelected) return;
                  setSelected(attempt.assessmentAttemptId);
                  setSelectedSummary(attempt);
                  setEvidence(undefined);
                },
              }),
            }}
          />
          <button
            type="button"
            class="primary-button"
            disabled={busy() || selected() === ""}
            onClick={() => void recover()}
          >
            Recover selected Attempt
          </button>
          <Show when={selectedSummary()}>
            {(attempt) => (
              <p role="status">
                Selected: {attempt().assessmentTitle}, Attempt {attempt().assessmentAttemptNumber}.
              </p>
            )}
          </Show>
          <Show when={evidence()}>
            {(attempt) => (
              <section aria-label="Recovered Attempt evidence">
                <h3>
                  {attempt().assessmentTitle}: Attempt {attempt().assessmentAttemptNumber}
                </h3>
                <p role="status">
                  Retained evidence recovered. Original deletion cutoff:{" "}
                  {formatRecoveryInstant(
                    attempt().deleteDueAt,
                    formatDateTime,
                    "No retained deletion cutoff",
                  )}
                  .
                </p>
                <p>
                  Archived:{" "}
                  {formatRecoveryInstant(
                    attempt().studentDataArchivedAt,
                    formatDateTime,
                    "No retained archive time",
                  )}
                  ; started:{" "}
                  {formatRecoveryInstant(
                    attempt().startedAt,
                    formatDateTime,
                    "No retained start time",
                  )}
                  ; expiry:{" "}
                  {formatRecoveryInstant(attempt().expiresAt, formatDateTime, "No retained expiry")}
                  .
                </p>
                <Evidence label="Exact Attempt facts" text={attempt().attemptFactsText} />
                <Evidence label="Submission" text={attempt().submissionText} />
                <Show when={recognition.status()}>
                  <p role="status">{recognition.status()}</p>
                </Show>
                <RecoveredQuestions
                  attempt={attempt()}
                  questionTitles={recognition.titles().questions}
                />
              </section>
            )}
          </Show>
        </section>
      </Show>
    </section>
  );
}
