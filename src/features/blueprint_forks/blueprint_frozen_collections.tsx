// Read-only projections of saved Blueprint Assessments and their authored entries.

import { createMemo, For, Show, type JSX } from "solid-js";
import type { BlueprintComparisonSide } from "../../../generated/api/BlueprintComparisonSide";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import type { CanonicalBlueprintAssessmentEntry } from "../../../generated/api/CanonicalBlueprintAssessmentEntry";
import { emptyRecognitionTitles, type RecognitionTitleMaps } from "../../api/recognition_titles";
import { assessmentTypePresentation } from "../../assessment_type_presentation";
import type { RecordContent } from "../../components/data_sheet/record_content";
import type { RecordField } from "../../components/data_sheet/record_fields";
import {
  labeledQuestionRecognition,
  recognitionTitle,
  recognizedIdList,
} from "../recognition_label";
import { SearchBlock } from "../../components/search_block/search_block";
import { SearchControls } from "../../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../../components/search_block/local_complete_search_state";
import { assessmentDifferenceLabels, forkModuleLabel, settingLines } from "./blueprint_fork_model";

type FrozenAssessment = BlueprintComparisonSide["assessments"][number];
export type FrozenComparisonKind = "related" | "left" | "right";
export type FrozenComparisonRow = {
  readonly id: string;
  readonly kind: FrozenComparisonKind;
  readonly title: string;
  readonly placement: string;
  readonly sharedQuestions: string;
  readonly differences: string;
  readonly left?: FrozenAssessment;
  readonly right?: FrozenAssessment;
};

function placement(side: BlueprintComparisonSide, item: FrozenAssessment): string {
  const module = side.modules.find(
    (candidate) => candidate.blueprintModuleId === item.blueprintModuleId,
  );
  return `${forkModuleLabel(side, item.blueprintModuleId)}; Module position ${module ? module.position + 1 : "unknown"}; Assessment position ${item.position + 1}`;
}

/** Assessment identity comes from server relationships, independent of names and display order. */
export function frozenComparisonRows(
  view: BlueprintComparisonView,
  titles: RecognitionTitleMaps,
): Record<FrozenComparisonKind, ReadonlyArray<FrozenComparisonRow>> {
  const related: FrozenComparisonRow[] = view.assessmentRelationships.map((edge) => {
    const left = view.left.assessments.find(
      (item) => item.blueprintAssessmentId === edge.leftAssessmentId,
    );
    const right = view.right.assessments.find(
      (item) => item.blueprintAssessmentId === edge.rightAssessmentId,
    );
    return {
      id: JSON.stringify(["related", edge.leftAssessmentId, edge.rightAssessmentId]),
      kind: "related",
      title: `${left?.content.title ?? edge.leftAssessmentId} compared with ${right?.content.title ?? edge.rightAssessmentId}`,
      placement: `Left: ${left ? placement(view.left, left) : "Not present"}. Right: ${right ? placement(view.right, right) : "Not present"}.`,
      sharedQuestions: recognizedIdList(
        edge.sharedQuestionIds,
        titles.questions,
        "Question",
        "Question ID",
      ),
      differences:
        left && right
          ? assessmentDifferenceLabels(left, right, view.left, view.right)
          : "Assessment not present in this Revision.",
      left,
      right,
    };
  });
  function sideOnly(kind: "left" | "right"): FrozenComparisonRow[] {
    return view[kind].assessments
      .filter(
        (item) =>
          !view.assessmentRelationships.some(
            (edge) =>
              (kind === "left" ? edge.leftAssessmentId : edge.rightAssessmentId) ===
              item.blueprintAssessmentId,
          ),
      )
      .map((item) => ({
        id: JSON.stringify([kind, item.blueprintAssessmentId]),
        kind,
        title: item.content.title,
        placement: placement(view[kind], item),
        sharedQuestions: "None on the other side",
        differences: "No shared-Question relationship; this does not establish history.",
        ...(kind === "left" ? { left: item } : { right: item }),
      }));
  }
  return { related, left: sideOnly("left"), right: sideOnly("right") };
}

export function Settings(props: { readonly value: unknown }): JSX.Element {
  return (
    <dl class="blueprint-fork-settings">
      <For each={settingLines(props.value)}>
        {(line) => (
          <>
            <dt>{line.label}</dt>
            <dd>{line.value}</dd>
          </>
        )}
      </For>
    </dl>
  );
}

export type FrozenEntryRow = {
  readonly id: string;
  readonly position: number;
  readonly entry: CanonicalBlueprintAssessmentEntry;
  readonly title: string;
  readonly identifier: string;
  readonly points: string;
  readonly selectionCount: number | null;
};

/** Saved ordinal plus immutable identity preserves repeated Question pins and Pool entries. */
export function frozenEntryRows(
  entries: ReadonlyArray<CanonicalBlueprintAssessmentEntry>,
  titles: RecognitionTitleMaps,
): FrozenEntryRow[] {
  return entries.map((entry, index) => {
    if (entry.kind === "fixed") {
      const tuple = entry.published_question_revision_tuple;
      const line = labeledQuestionRecognition(
        titles.questions.get(tuple.publishedQuestionId),
        tuple.publishedQuestionId,
        tuple.revisionNumber,
      );
      return {
        id: JSON.stringify([index, "fixed", tuple.publishedQuestionId, tuple.revisionNumber]),
        position: index + 1,
        entry,
        title: line.title,
        identifier: line.identifier,
        points: `${entry.points_possible} points`,
        selectionCount: null,
      };
    }
    return {
      id: JSON.stringify([index, "pool", entry.question_pool_id]),
      position: index + 1,
      entry,
      title: recognitionTitle(titles.pools.get(entry.question_pool_id), "Question Pool"),
      identifier: `Question Pool ID ${entry.question_pool_id}`,
      points: `${entry.points_per_item} points per Question`,
      selectionCount: entry.selection_count,
    };
  });
}

export const frozenEntryFields: ReadonlyArray<RecordField<FrozenEntryRow, never, "kind">> = [
  {
    id: "title",
    label: "Question or Pool",
    role: "identity",
    value: (row) => row.title,
    defaultColumn: { priority: 0, width: "20rem" },
  },
  {
    id: "position",
    label: "Authored position",
    value: (row) => row.position,
    defaultColumn: { priority: 1, width: "9rem" },
  },
  {
    id: "identifier",
    label: "Saved identity",
    value: (row) => row.identifier,
    defaultColumn: { priority: 2, width: "20rem" },
  },
  {
    id: "kind",
    label: "Entry kind",
    filterKey: "kind",
    value: (row) => row.entry.kind,
    defaultColumn: { priority: 3, width: "9rem" },
  },
  {
    id: "points",
    label: "Points",
    value: (row) => row.points,
    defaultColumn: { priority: 4, width: "12rem" },
  },
  {
    id: "selection_count",
    label: "Pool selections",
    value: (row) => row.selectionCount,
    defaultColumn: { priority: 5, width: "10rem" },
  },
  {
    id: "scoring_rule",
    label: "Scoring rule",
    value: (row) => row.entry.scoring_rule,
    defaultColumn: { priority: 6, width: "12rem" },
  },
  {
    id: "question_attempt_limit",
    label: "Question attempt limit",
    value: (row) =>
      settingLines({ question_attempt_limit: row.entry.question_attempt_limit })
        .map((line) => line.value)
        .join("; "),
    defaultColumn: { priority: 7, width: "12rem" },
  },
  {
    id: "question_attempt_time_limit",
    label: "Question attempt time limit",
    value: (row) =>
      settingLines({ question_attempt_time_limit: row.entry.question_attempt_time_limit })
        .map((line) => line.value)
        .join("; "),
    defaultColumn: { priority: 8, width: "12rem" },
  },
];

export function frozenEntryKeyword(row: FrozenEntryRow): string {
  return `${row.position} ${row.title} ${row.identifier} ${row.entry.kind} ${row.points} ${row.selectionCount ?? ""} ${settingLines(
    row.entry,
  )
    .map((line) => `${line.label} ${line.value}`)
    .join(" ")}`;
}

/** Display queries operate on a projection; the saved entry payload and authored order stay intact. */
export function FrozenAssessmentEntries(props: {
  readonly entries: ReadonlyArray<CanonicalBlueprintAssessmentEntry>;
  readonly titles: RecognitionTitleMaps;
}): JSX.Element {
  const rows = createMemo(() => frozenEntryRows(props.entries, props.titles));
  function content(row: FrozenEntryRow): RecordContent {
    return {
      title: row.title,
      details: [
        { kind: "text", label: "Authored position", value: String(row.position) },
        { kind: "text", label: "Saved identity", value: row.identifier },
        { kind: "text", label: "Points", value: row.points },
        ...(row.selectionCount === null
          ? []
          : [
              {
                kind: "text" as const,
                label: "Select Questions",
                value: String(row.selectionCount),
              },
            ]),
        ...settingLines({
          scoring_rule: row.entry.scoring_rule,
          question_attempt_limit: row.entry.question_attempt_limit,
          question_attempt_time_limit: row.entry.question_attempt_time_limit,
        }).map((line) => ({ kind: "text" as const, label: line.label, value: line.value })),
      ],
      actions: [],
    };
  }
  const search = createLocalCompleteSearch(
    { rows: () => rows() ?? [] },
    { fields: frozenEntryFields, rowId: (row) => row.id, keyword: frozenEntryKeyword, content },
  );
  return (
    <SearchBlock
      layout="above"
      controls={<SearchControls state={search.state} textLabel="Saved Question, Pool, or policy" />}
      state={search.state}
      collection={{
        ariaLabel: "Questions and Pools in authored order",
        fields: frozenEntryFields,
        query: { kind: search.query.kind, filters: search.query.filters },
        emptyState: { title: "No Questions or Pools match." },
      }}
    />
  );
}

export function AssessmentSnapshot(props: {
  readonly snapshot: FrozenAssessment | undefined;
  readonly side: BlueprintComparisonSide;
  readonly titles?: RecognitionTitleMaps;
}): JSX.Element {
  const titles = (): RecognitionTitleMaps => props.titles ?? emptyRecognitionTitles();
  return (
    <Show when={props.snapshot} fallback={<p>Not present in this Revision.</p>}>
      {(snapshot) => (
        <>
          {/* ASVS 1.2.1: saved authored strings are escaped JSX text. */}
          <h4>{snapshot().content.title}</h4>
          <p>{assessmentTypePresentation(snapshot().content.assessment_type).label}</p>
          <p>Module: {placement(props.side, snapshot())}.</p>
          <p class="blueprint-fork-instructions">
            {snapshot().content.instructions || "No instructions."}
          </p>
          <p>
            Question IDs in this Assessment:{" "}
            {recognizedIdList(
              snapshot().questionIds,
              titles().questions,
              "Question",
              "Question ID",
            )}
            .
          </p>
          <h5>Questions and Pools in authored order</h5>
          <FrozenAssessmentEntries entries={snapshot().content.entries} titles={titles()} />
          <h5>Assessment Properties</h5>
          <Settings value={snapshot().content.defaults} />
        </>
      )}
    </Show>
  );
}
