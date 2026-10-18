// Complete local collections for one immutable, exact saved Blueprint Revision.
import { createMemo, For, Show, createSignal, type JSX } from "solid-js";

import type { BlueprintAssessmentContentView } from "../../../generated/api/BlueprintAssessmentContentView";
import type { BlueprintAssessmentEntryView } from "../../../generated/api/BlueprintAssessmentEntryView";
import type { BlueprintRevisionView } from "../../../generated/api/BlueprintRevisionView";
import type { BlueprintCourseClient } from "../../api/blueprint_course";
import type { RecognitionTitleMaps } from "../../api/recognition_titles";
import {
  assessmentDurationDefaultDescription,
  assessmentDurationDisplay,
} from "../../assessment_duration";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import type { RecordContent } from "../../components/data_sheet/record_content";
import type { RecordField } from "../../components/data_sheet/record_fields";
import { labeledQuestionRecognition, recognitionTitle } from "../recognition_label";
import { recognitionTitlesResource } from "../recognition_titles_load";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import {
  blueprintStructureAssessmentContent,
  blueprintStructureAssessmentFields,
  blueprintStructureModuleContent,
  blueprintStructureModuleFields,
  type BlueprintStructureAssessment,
  type BlueprintStructureModule,
} from "./blueprint_course_structure_records";

/** Frozen views carry saved IDs directly; they are never editable choice DTOs. */
export function historicalModules(
  revision: BlueprintRevisionView,
): ReadonlyArray<BlueprintStructureModule> {
  return revision.modules.map((module, moduleIndex) => ({
    id: module.blueprint_module_id,
    moduleIndex,
    label: module.label,
    context: `${moduleIndex + 1}. ${module.label}`,
    assessmentCount: module.assessments.length,
  }));
}

/** Display queries retain each Assessment's original Module and Assessment positions. */
export function historicalAssessments(
  revision: BlueprintRevisionView,
): ReadonlyArray<BlueprintStructureAssessment> {
  return revision.modules.flatMap((module, moduleIndex) =>
    module.assessments.map((assessment, assessmentIndex) => ({
      id: assessment.blueprint_assessment_id,
      moduleIndex,
      assessmentIndex,
      moduleContext: `${moduleIndex + 1}. ${module.label}`,
      title: assessment.content.title,
      assessmentType: assessment.content.assessment_type,
      entryCount: assessment.content.entries.length,
    })),
  );
}

export type HistoricalEntry = {
  readonly id: string;
  readonly position: number;
  readonly entry: BlueprintAssessmentEntryView;
  readonly title: string;
  readonly kind: "Question" | "Question Pool";
  readonly objectId: string;
  readonly revisionNumber: number | null;
  readonly selectionCount: number;
  readonly points: string;
  readonly timeLimit: string;
};

/** Saved ordinal distinguishes repeated exact Questions and repeated Pools. */
export function historicalEntries(
  content: BlueprintAssessmentContentView,
  titles: RecognitionTitleMaps,
): ReadonlyArray<HistoricalEntry> {
  return content.entries.map((entry, index) => {
    const tuple = entry.kind === "fixed" ? entry.question.published_question_revision_tuple : null;
    const objectId =
      entry.kind === "fixed"
        ? entry.question.published_question_revision_tuple.publishedQuestionId
        : entry.question_pool_id;
    const revisionNumber = tuple?.revisionNumber ?? null;
    const title =
      tuple === null
        ? recognitionTitle(titles.pools.get(objectId), "Question Pool")
        : labeledQuestionRecognition(titles.questions.get(objectId), objectId, tuple.revisionNumber)
            .title;
    const limit = entry.question_attempt_time_limit;
    const timeLimit =
      limit.kind === "limited"
        ? `${assessmentDurationDisplay(limit.seconds)} (${limit.graceSeconds === 0 ? "no grace" : `${assessmentDurationDisplay(limit.graceSeconds)} grace`})`
        : "unlimited";
    return {
      id: `${index + 1}-${entry.kind}-${objectId}${revisionNumber === null ? "" : `-${revisionNumber}`}`,
      position: index + 1,
      entry,
      title,
      kind: entry.kind === "fixed" ? "Question" : "Question Pool",
      objectId,
      revisionNumber,
      selectionCount: entry.kind === "fixed" ? 1 : entry.selection_count,
      points: String(entry.kind === "fixed" ? entry.points_possible : entry.points_per_item),
      timeLimit,
    };
  });
}

// Assessment entry display queries preserve saved order: no sortable columns.
export const historicalEntryFields: ReadonlyArray<RecordField<HistoricalEntry, never, "kind">> = [
  {
    id: "title",
    role: "identity",
    label: "Library Object",
    value: (row) => row.title,
    defaultColumn: { priority: 0, width: "18rem" },
  },
  {
    id: "position",
    label: "Saved position",
    value: (row) => row.position,
    defaultColumn: { priority: 1, width: "8rem" },
  },
  {
    id: "kind",
    label: "Kind",
    filterKey: "kind",
    value: (row) => row.kind,
    defaultColumn: { priority: 2, width: "10rem" },
  },
  {
    id: "object_id",
    label: "Public ID",
    value: (row) => row.objectId,
    defaultColumn: { priority: 3, width: "10rem" },
  },
  {
    id: "revision",
    label: "Question Revision",
    value: (row) => row.revisionNumber,
    defaultColumn: { priority: 4, width: "10rem" },
  },
  {
    id: "selection_count",
    label: "Questions selected",
    value: (row) => row.selectionCount,
    defaultColumn: { priority: 5, width: "10rem" },
  },
  {
    id: "points",
    label: "Points per Question",
    value: (row) => row.points,
    defaultColumn: { priority: 6, width: "10rem" },
  },
  {
    id: "scoring",
    label: "Scoring",
    value: (row) => row.entry.scoring_rule,
    defaultColumn: { priority: 7, width: "10rem", visible: false },
  },
  {
    id: "attempt_limit",
    label: "Question Attempt limit",
    value: (row) => row.entry.question_attempt_limit.maxAttempts ?? "unlimited",
    defaultColumn: { priority: 8, width: "12rem", visible: false },
  },
  {
    id: "time_limit",
    label: "Time limit",
    value: (row) => row.timeLimit,
    defaultColumn: { priority: 9, width: "14rem", visible: false },
  },
];

export function historicalEntryContent(row: HistoricalEntry): RecordContent {
  return {
    title: row.title,
    details: [
      { kind: "text", label: "Saved position", value: String(row.position) },
      { kind: "text", label: `${row.kind} ID`, value: row.objectId },
      ...(row.revisionNumber === null
        ? []
        : [{ kind: "text" as const, label: "Revision", value: String(row.revisionNumber) }]),
      { kind: "text", label: "Questions selected", value: String(row.selectionCount) },
      {
        kind: "text",
        label: row.entry.kind === "fixed" ? "Points" : "Points per Question",
        value: row.points,
      },
      { kind: "text", label: "Scoring", value: row.entry.scoring_rule },
      {
        kind: "text",
        label: "Question Attempt limit",
        value: String(row.entry.question_attempt_limit.maxAttempts ?? "unlimited"),
      },
      { kind: "text", label: "Time limit", value: row.timeLimit },
    ],
    actions: [],
  };
}

/** Independent complete collections stay mounted while one saved Assessment is inspected. */
export function BlueprintHistoryContent(props: {
  readonly client: BlueprintCourseClient;
  readonly revision: BlueprintRevisionView;
}): JSX.Element {
  const [selectedAssessment, setSelectedAssessment] = createSignal<BlueprintStructureAssessment>();
  let originalTrigger: HTMLButtonElement | undefined;
  let overview: HTMLElement | undefined;
  let inspection: HTMLElement | undefined;
  const modules = createMemo(() => historicalModules(props.revision));
  const assessments = createMemo(() => historicalAssessments(props.revision));
  const assessmentSearch = createLocalCompleteSearch(
    { rows: assessments },
    {
      fields: blueprintStructureAssessmentFields,
      rowId: (row) => row.id,
      keyword: (row) =>
        `${row.title} ${row.moduleContext} ${assessmentTypePresentation(row.assessmentType).label}`,
      compare: (key, left, right) =>
        key === "module" ? left.moduleIndex - right.moduleIndex : undefined,
      content: (row) =>
        blueprintStructureAssessmentContent(row, {
          id: "view_historical_assessment",
          kind: "command",
          label: "View historical assessment",
          onClick: (event) => {
            originalTrigger = event.currentTarget;
            setSelectedAssessment(row);
            queueMicrotask(() => inspection?.focus());
          },
        }),
    },
  );
  const moduleSearch = createLocalCompleteSearch(
    { rows: modules },
    {
      fields: blueprintStructureModuleFields,
      rowId: (row) => row.id,
      keyword: (row) => row.label,
      content: (row) =>
        blueprintStructureModuleContent(row, {
          id: "view_assessments",
          kind: "command",
          label: "View Assessments",
          onClick: () => assessmentSearch.query.filters?.onChange("module", [row.context]),
        }),
    },
  );
  function returnToAssessments(): void {
    setSelectedAssessment(undefined);
    queueMicrotask(() => (originalTrigger?.isConnected ? originalTrigger : overview)?.focus());
  }
  return (
    <>
      <div
        hidden={selectedAssessment() !== undefined}
        class="blueprint-course-structure-collections"
      >
        {/* ASVS 1.2.1: authored labels and titles use escaped shared record fields. */}
        <section aria-label="Historical Blueprint Module index">
          <h4>Modules</h4>
          <SearchBlock
            layout="above"
            state={moduleSearch.state}
            controls={
              <SearchControls state={moduleSearch.state} textLabel="Search historical Modules" />
            }
            collection={{
              ariaLabel: "Historical Blueprint Modules",
              initialMode: "compact",
              fields: blueprintStructureModuleFields,
              query: moduleSearch.query,
              emptyState: {
                title: "No matching Modules",
                message: "This saved Revision has no Modules matching your search.",
              },
            }}
          />
        </section>
        <section
          aria-label="Historical Blueprint Assessment overview"
          tabindex="-1"
          ref={(element) => (overview = element)}
        >
          <h4>Assessments</h4>
          <SearchBlock
            layout="above"
            state={assessmentSearch.state}
            controls={
              <SearchControls
                state={assessmentSearch.state}
                textLabel="Search historical Assessments"
              />
            }
            collection={{
              ariaLabel: "Historical Blueprint Assessments",
              initialMode: "compact",
              fields: blueprintStructureAssessmentFields,
              query: assessmentSearch.query,
              emptyState: {
                title: "No matching Assessments",
                message: "Try another search or Module filter.",
              },
            }}
          />
        </section>
      </div>
      <Show when={selectedAssessment()} keyed>
        {(row) => (
          <section
            aria-label="Read-only historical assessment"
            tabindex="-1"
            ref={(element) => (inspection = element)}
          >
            <button type="button" class="quiet-action" onClick={returnToAssessments}>
              Return to historical assessment list
            </button>
            <p>
              Module: {row.moduleContext}; saved Assessment position: {row.assessmentIndex + 1}
            </p>
            <HistoricalAssessment
              client={props.client}
              content={
                props.revision.modules[row.moduleIndex]!.assessments[row.assessmentIndex]!.content
              }
            />
          </section>
        )}
      </Show>
    </>
  );
}

function policyLabel(name: string): string {
  const words = name.replace(/([a-z])([A-Z])/gu, "$1 $2").replace(/_/gu, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function historicalAssessmentDefaultDuration(content: BlueprintAssessmentContentView): string {
  const seconds = content.defaults.assessment_attempt_time_limit_seconds;
  if (seconds === null) {
    const count = content.entries.reduce(
      (total, entry) => total + (entry.kind === "fixed" ? 1 : entry.selection_count),
      0,
    );
    return assessmentDurationDefaultDescription(count);
  }
  return assessmentDurationDisplay(seconds);
}

function HistoricalAssessment(props: {
  readonly client: BlueprintCourseClient;
  readonly content: BlueprintAssessmentContentView;
}): JSX.Element {
  const recognition = recognitionTitlesResource(
    () => props.client,
    () => ({
      questionIds: props.content.entries.flatMap((entry) =>
        entry.kind === "fixed"
          ? [entry.question.published_question_revision_tuple.publishedQuestionId]
          : [],
      ),
      poolIds: props.content.entries.flatMap((entry) =>
        entry.kind === "pool" ? [entry.question_pool_id] : [],
      ),
    }),
  );
  const entries = createMemo(() => historicalEntries(props.content, recognition.titles()));
  const search = createLocalCompleteSearch(
    { rows: entries },
    {
      fields: historicalEntryFields,
      rowId: (row) => row.id,
      keyword: (row) =>
        `${row.title} ${row.kind} ${row.objectId} ${row.revisionNumber ?? ""} ${row.position}`,
      content: historicalEntryContent,
    },
  );
  return (
    <>
      <h4>{props.content.title}</h4>
      <p>Assessment type: {assessmentTypePresentation(props.content.assessment_type).label}</p>
      {/* ASVS 1.2.1: authored instructions remain escaped plain text, never HTML. */}
      <p style={{ "white-space": "pre-wrap" }}>{props.content.instructions}</p>
      <section aria-label="Historical Assessment entry collection">
        <h5>Question entries</h5>
        <Show when={recognition.status()}>
          <p role="status">{recognition.status()}</p>
        </Show>
        <SearchBlock
          layout="above"
          state={search.state}
          controls={<SearchControls state={search.state} textLabel="Search historical entries" />}
          collection={{
            ariaLabel: "Historical Question and Pool entries",
            initialMode: "compact",
            fields: historicalEntryFields,
            query: { kind: search.query.kind, filters: search.query.filters },
            emptyState: {
              title: "No matching Question entries",
              message: "This saved Assessment has no entries matching your search or filter.",
            },
          }}
        />
      </section>
      <h5>Assessment defaults</h5>
      <dl>
        <dt>Assessment Attempt time limit</dt>
        <dd>{historicalAssessmentDefaultDuration(props.content)}</dd>
        <dt>Assessment Attempt limit</dt>
        <dd>{props.content.defaults.assessment_attempt_limit ?? "Unlimited"}</dd>
        <dt>Late work rule</dt>
        <dd>{props.content.defaults.late_work_rule}</dd>
        <dt>Student feedback release rule</dt>
        <dd>
          <dl>
            <For each={Object.entries(props.content.defaults.student_feedback_release_rule)}>
              {([name, value]) => (
                <>
                  <dt>{policyLabel(name)}</dt>
                  <dd>{value}</dd>
                </>
              )}
            </For>
          </dl>
        </dd>
        <For each={Object.entries(props.content.defaults.activity_rules)}>
          {([name, value]) => (
            <>
              <dt>{policyLabel(name)}</dt>
              <dd>{value}</dd>
            </>
          )}
        </For>
      </dl>
    </>
  );
}
