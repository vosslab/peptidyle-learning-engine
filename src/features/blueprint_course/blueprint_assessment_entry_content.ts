// Recognition heading for one Blueprint Assessment entry.

import type { BlueprintAssessmentContentInput } from "../../../generated/api/BlueprintAssessmentContentInput";
import type { RecordContent } from "../../components/record_list/record_list";
import type { RecognitionTitleMaps } from "../../api/recognition_titles";
import { recognitionTitle } from "../recognition_label";

type EntryInput = BlueprintAssessmentContentInput["entries"][number];

/** Title for recognition, with the public ID kept as a labeled fact. */
export function blueprintAssessmentEntryContent(
  entry: EntryInput,
  titles: RecognitionTitleMaps,
  remove: (() => void) | undefined,
): RecordContent {
  const removeAction =
    remove === undefined
      ? []
      : [{ id: "remove-entry", kind: "command" as const, label: "Remove", onClick: remove }];
  if (entry.kind === "fixed") {
    const revision = entry.published_question_revision_tuple;
    return {
      title: recognitionTitle(titles.questions.get(revision.publishedQuestionId), "Question"),
      details: [
        { kind: "text", label: "Question ID", value: revision.publishedQuestionId },
        { kind: "text", label: "Revision", value: String(revision.revisionNumber) },
        { kind: "text", label: "Points possible", value: entry.points_possible },
        { kind: "text", label: "Scoring", value: entry.scoring_rule },
      ],
      actions: removeAction,
    };
  }
  return {
    title: recognitionTitle(titles.pools.get(entry.pool.question_pool_id), "Question Pool"),
    details: [
      { kind: "text", label: "Question Pool ID", value: entry.pool.question_pool_id },
      { kind: "text", label: "Edit", value: String(entry.pool.question_pool_edit_number) },
      {
        kind: "text",
        label: "Draw each Assessment Attempt",
        value: String(entry.selection_count),
      },
      { kind: "text", label: "Points per Question", value: entry.points_per_item },
      { kind: "text", label: "Scoring", value: entry.scoring_rule },
    ],
    actions: removeAction,
  };
}

/** Accessible name for moving or labeling one entry. */
export function blueprintAssessmentEntryLabel(
  entry: EntryInput,
  titles: RecognitionTitleMaps,
): string {
  if (entry.kind === "fixed") {
    const revision = entry.published_question_revision_tuple;
    return `${recognitionTitle(titles.questions.get(revision.publishedQuestionId), "Question")} (Question ID ${revision.publishedQuestionId}, Revision ${revision.revisionNumber})`;
  }
  return `${recognitionTitle(titles.pools.get(entry.pool.question_pool_id), "Question Pool")} (Question Pool ID ${entry.pool.question_pool_id}, Edit ${entry.pool.question_pool_edit_number})`;
}
