import { For, Show, createMemo, createSignal, createUniqueId, type JSX } from "solid-js";
import type {
  AssessmentBlueprintUpdateContent,
  AssessmentBlueprintUpdateEntry,
  LiveAssessmentWorkspace,
} from "../../api/assessment_release";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { AssessmentEntryAvailability } from "../../../generated/api/AssessmentEntryAvailability";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import {
  assessmentDurationDefaultDescription,
  assessmentDurationDisplay,
} from "../../assessment_duration";
import { RecordDetails } from "../../components/data_sheet/record_content";
import type { RecordContent } from "../../components/data_sheet/record_content";
import type { RecordField } from "../../components/data_sheet/data_sheet_fields";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";

/** Projects only reusable teaching content; Course delivery state stays outside the update. */
export function currentBlueprintUpdateContent(
  current: LiveAssessmentWorkspace,
): AssessmentBlueprintUpdateContent {
  return {
    assessmentType: current.assessmentType,
    title: current.title,
    instructions: current.instructions,
    defaults: {
      assessment_attempt_time_limit_seconds: current.assessmentAttemptTimeLimitSeconds,
      assessment_attempt_limit: current.attemptLimit,
      late_work_rule: current.lateWorkRule,
      activity_rules: current.activityRules,
      student_feedback_release_rule: current.studentFeedbackReleaseRule,
    },
    entries: current.entries,
  };
}

/** Shared semantic presentation for each current or proposed ordered Assessment entry. */
function assessmentEntryContent(props: {
  readonly entry: AssessmentBlueprintUpdateEntry & {
    readonly availability?: AssessmentEntryAvailability;
  };
  readonly questionTitle: (
    publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
  ) => string;
  readonly poolTitle: (questionPoolId: string) => string;
  readonly description: (publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple) => string;
  readonly poolRole: "assessmentOwned" | "librarySource";
}): RecordContent {
  const timeLimit = (): string =>
    props.entry.questionAttemptTimeLimit.kind === "unlimited"
      ? "No Question time limit"
      : `${props.entry.questionAttemptTimeLimit.seconds}s Question time limit; ${props.entry.questionAttemptTimeLimit.graceSeconds}s grace`;
  const commonDetails: ReadonlyArray<RecordContent["details"][number]> = [
    ...(props.entry.availability === undefined
      ? []
      : [{ kind: "text" as const, label: "Availability", value: props.entry.availability }]),
    { kind: "text", label: "Scoring", value: `${props.entry.scoringRule} scoring` },
    {
      kind: "text",
      label: "Question Attempts",
      value: `${props.entry.questionAttemptLimit.maxAttempts ?? "Unlimited"}`,
    },
    { kind: "text", label: "Question time limit", value: timeLimit() },
  ];

  if (props.entry.kind === "fixedQuestion") {
    const revision = props.entry.publishedQuestionRevisionTuple;
    return {
      title: props.questionTitle(revision),
      description: props.description(revision),
      details: [
        { kind: "text", label: "Question ID", value: revision.publishedQuestionId },
        { kind: "text", label: "Revision", value: String(revision.revisionNumber) },
        { kind: "text", label: "Points", value: `${props.entry.pointsPossible} points` },
        ...commonDetails,
      ],
      actions: [],
    };
  }

  const poolLabel =
    props.poolRole === "assessmentOwned"
      ? "Current Assessment Pool reference"
      : "Library source Question Pool";
  return {
    title: props.poolTitle(props.entry.questionPoolId),
    description: poolLabel,
    details: [
      { kind: "text", label: "Question Pool ID", value: props.entry.questionPoolId },
      { kind: "text", label: "Selection count", value: `${props.entry.selectionCount}` },
      {
        kind: "text",
        label: "Points per Question",
        value: `${props.entry.pointsPerItem} points`,
      },
      ...commonDetails,
    ],
    actions: [],
  };
}

const ACTIVITY_LABELS = [
  ["partialCreditEnabled", "Partial credit"],
  ["questionVariationRule", "Question variation"],
  ["assessmentQuestionOrderRule", "Question order"],
] as const;
const FEEDBACK_LABELS = [
  ["question_answer", "Correct answer"],
  ["question_answer_explanation", "Question answer explanation"],
  ["class_statistics", "Class statistics"],
  ["hints", "Hints"],
  ["worked_solutions", "Worked Solutions"],
] as const;

function settingCopy(value: string): string {
  return value.replace(/_/gu, " ").replace(/([a-z])([A-Z])/gu, "$1 $2");
}

function activitySettingCopy(value: boolean | string): string {
  return typeof value === "boolean" ? (value ? "Enabled" : "Disabled") : settingCopy(value);
}

type BlueprintEntryRow = {
  readonly entry: AssessmentBlueprintUpdateEntry & {
    readonly availability?: AssessmentEntryAvailability;
  };
  readonly index: number;
};
type BlueprintEntryFilter = "kind" | "scoring" | "availability";

function blueprintEntryId(row: BlueprintEntryRow): string {
  return `${row.entry.kind}:${row.index}`;
}

/** One complete labeled presentation, reused for current and proposed content without HTML interpretation. */
export function AssessmentBlueprintContentSummary(props: {
  readonly heading: string;
  readonly content: AssessmentBlueprintUpdateContent;
  readonly questionTitle: (
    publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
  ) => string;
  readonly poolTitle: (questionPoolId: string) => string;
  readonly description: (publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple) => string;
  readonly poolRole: "assessmentOwned" | "librarySource";
}): JSX.Element {
  const rows = createMemo(() => props.content.entries.map((entry, index) => ({ entry, index })));
  const [inspectedId, setInspectedId] = createSignal<string | null>(null);
  const inspectedRow = createMemo(() =>
    rows().find((row) => blueprintEntryId(row) === inspectedId()),
  );
  const detailsId = `blueprint-entry-details-${createUniqueId()}`;
  let detailsSection: HTMLElement | undefined;
  let inspectTrigger: HTMLButtonElement | undefined;
  function entryContent(row: BlueprintEntryRow): RecordContent {
    const content = assessmentEntryContent({
      entry: row.entry,
      questionTitle: props.questionTitle,
      poolTitle: props.poolTitle,
      description: props.description,
      poolRole: props.poolRole,
    });
    return {
      ...content,
      details: [
        { kind: "text", label: "Position", value: String(row.index + 1) },
        ...content.details,
      ],
    };
  }
  const fields: ReadonlyArray<RecordField<BlueprintEntryRow, never, BlueprintEntryFilter>> = [
    {
      id: "position",
      label: "Position",
      value: (row) => row.index + 1,
      defaultColumn: { priority: 1, width: "6rem" },
    },
    {
      id: "title",
      label: "Question or Question Pool",
      role: "identity",
      value: (row) => entryContent(row).title,
      defaultColumn: { priority: 0, width: "20rem" },
    },
    {
      id: "kind",
      label: "Entry type",
      filterKey: "kind",
      value: (row) => (row.entry.kind === "fixedQuestion" ? "Question" : "Question Pool"),
      defaultColumn: { priority: 2, width: "10rem" },
    },
    {
      id: "points",
      label: "Points",
      value: (row) =>
        row.entry.kind === "fixedQuestion"
          ? `${row.entry.pointsPossible} points`
          : `${row.entry.pointsPerItem} points per Question`,
      defaultColumn: { priority: 3, width: "12rem" },
    },
    {
      id: "selection_count",
      label: "Selection count",
      value: (row) => (row.entry.kind === "questionPool" ? row.entry.selectionCount : null),
      defaultColumn: { priority: 4, width: "9rem", visible: false },
    },
    {
      id: "scoring",
      label: "Scoring",
      filterKey: "scoring",
      value: (row) => row.entry.scoringRule,
      defaultColumn: { priority: 5, width: "10rem", visible: false },
    },
    {
      id: "availability",
      label: "Availability",
      filterKey: "availability",
      value: (row) => row.entry.availability ?? null,
      defaultColumn: { priority: 6, width: "10rem", visible: false },
    },
  ];
  const entrySearch = createLocalCompleteSearch(
    { rows: rows },
    {
      fields,
      rowId: blueprintEntryId,
      keyword: (row) => {
        const content = entryContent(row);
        const details = content.details.map((fact) => (fact.kind === "text" ? fact.value : ""));
        return `${row.index + 1} ${row.entry.kind} ${content.title} ${content.description ?? ""} ${details.join(" ")}`;
      },
      content: (row): RecordContent => ({
        ...entryContent(row),
        actions: [
          {
            id: "inspect",
            kind: "command",
            label: "Inspect",
            title: `Inspect entry ${row.index + 1}: ${entryContent(row).title}`,
            controls: detailsId,
            expanded: inspectedId() === blueprintEntryId(row),
            onClick: (event): void => {
              inspectTrigger = event.currentTarget;
              setInspectedId(blueprintEntryId(row));
              queueMicrotask(() => detailsSection?.focus());
            },
          },
        ],
      }),
    },
  );
  const defaults = (): AssessmentBlueprintUpdateContent["defaults"] => props.content.defaults;
  const assessmentDuration = (): string => {
    const seconds = defaults().assessment_attempt_time_limit_seconds;
    if (seconds !== null) return assessmentDurationDisplay(seconds);
    return assessmentDurationDefaultDescription(
      props.content.entries.reduce(
        (count, entry) => count + (entry.kind === "fixedQuestion" ? 1 : entry.selectionCount),
        0,
      ),
    );
  };
  // ASVS 1.2.1: titles, instructions, and Question descriptions remain escaped text nodes.
  return (
    <section class="assessment-editor-panel" aria-label={props.heading}>
      <h4>{props.heading}</h4>
      <dl class="assessment-facts">
        <div>
          <dt>Assessment Type</dt>
          <dd>{assessmentTypePresentation(props.content.assessmentType).label}</dd>
        </div>
        <div>
          <dt>Title</dt>
          <dd>{props.content.title}</dd>
        </div>
        <div>
          <dt>Student instructions</dt>
          <dd class="plain-text-instructions">{props.content.instructions || "No instructions"}</dd>
        </div>
        <div>
          <dt>Assessment Attempt time limit</dt>
          <dd>{assessmentDuration()}</dd>
        </div>
        <div>
          <dt>Assessment Attempt limit</dt>
          <dd>{defaults().assessment_attempt_limit ?? "Unlimited"}</dd>
        </div>
        <div>
          <dt>Late work</dt>
          <dd>{settingCopy(defaults().late_work_rule)}</dd>
        </div>
        <For each={ACTIVITY_LABELS}>
          {([field, label]) => (
            <div>
              <dt>{label}</dt>
              <dd>{activitySettingCopy(defaults().activity_rules[field])}</dd>
            </div>
          )}
        </For>
        <For each={FEEDBACK_LABELS}>
          {([field, label]) => (
            <div>
              <dt>{label}</dt>
              <dd>{settingCopy(defaults().student_feedback_release_rule[field])}</dd>
            </div>
          )}
        </For>
      </dl>
      <h5>Ordered Questions and Question Pools</h5>
      <SearchBlock
        layout="above"
        controls={
          <SearchControls
            state={entrySearch.state}
            textLabel={`Search ${props.heading} Questions and Question Pools`}
          />
        }
        state={entrySearch.state}
        collection={{
          ariaLabel: `${props.heading} ordered Questions and Question Pools`,
          ...entrySearch.collection,
          // Saved Assessment order is fixed; filtering preserves original positions.
          query: { filters: entrySearch.collection.query.filters },
          get emptyState() {
            return {
              title:
                entrySearch.state.query().text || entrySearch.state.query().filters.size
                  ? "No Entries match your search."
                  : "No Entries.",
            };
          },
        }}
      />
      <Show when={inspectedRow()}>
        {(row) => (
          <section
            id={detailsId}
            ref={(element) => (detailsSection = element)}
            tabindex="-1"
            aria-label={`${props.heading} entry ${row().index + 1} details`}
          >
            <h5>
              Entry {row().index + 1}: {entryContent(row()).title}
            </h5>
            <p>{entryContent(row()).description}</p>
            <RecordDetails content={() => entryContent(row())} />
            <button
              type="button"
              class="quiet-action"
              onClick={() => {
                setInspectedId(null);
                if (inspectTrigger?.isConnected) inspectTrigger.focus();
              }}
            >
              Close entry details
            </button>
          </section>
        )}
      </Show>
    </section>
  );
}
