// Human title plus the public ID a person must recognize or copy.

import type { BlueprintComparisonView } from "../../generated/api/BlueprintComparisonView";
import type { CanonicalBlueprintCourse } from "../../generated/api/CanonicalBlueprintCourse";

/** Uses the lineage title when it is present and a plain record noun when it is not. */
export function recognitionTitle(title: string | undefined, fallback: string): string {
  const trimmed = title?.trim() ?? "";
  return trimmed.length > 0 ? trimmed : fallback;
}

/** Question and Pool public IDs shown by one Blueprint comparison. */
export function comparisonRecognitionIds(view: BlueprintComparisonView): {
  readonly questionIds: readonly string[];
  readonly poolIds: readonly string[];
} {
  const questionIds = new Set<string>();
  const poolIds = new Set<string>();
  for (const id of view.sharedQuestionIds) questionIds.add(id);
  for (const id of view.leftOnlyQuestionIds) questionIds.add(id);
  for (const id of view.rightOnlyQuestionIds) questionIds.add(id);
  for (const side of [view.left, view.right]) {
    for (const assessment of side.assessments) {
      for (const id of assessment.questionIds) questionIds.add(id);
      for (const entry of assessment.content.entries) {
        if (entry.kind === "fixed") {
          questionIds.add(entry.published_question_revision_tuple.publishedQuestionId);
        } else {
          poolIds.add(entry.question_pool_id);
        }
      }
    }
  }
  return { questionIds: [...questionIds], poolIds: [...poolIds] };
}

/** Question and Pool public IDs in one canonical Blueprint Course document. */
export function canonicalCourseRecognitionIds(course: CanonicalBlueprintCourse): {
  readonly questionIds: readonly string[];
  readonly poolIds: readonly string[];
} {
  const questionIds = new Set<string>();
  const poolIds = new Set<string>();
  for (const module of course.modules) {
    for (const assessment of module.assessments) {
      for (const entry of assessment.entries) {
        if (entry.kind === "fixed") {
          questionIds.add(entry.published_question_revision_tuple.publishedQuestionId);
        } else {
          poolIds.add(entry.question_pool_id);
        }
      }
    }
  }
  return { questionIds: [...questionIds], poolIds: [...poolIds] };
}

/** Distinct public IDs from several recognition surfaces. */
export function mergeRecognitionIdSets(
  ...parts: ReadonlyArray<{
    readonly questionIds: readonly string[];
    readonly poolIds: readonly string[];
  }>
): { readonly questionIds: readonly string[]; readonly poolIds: readonly string[] } {
  const questionIds = new Set<string>();
  const poolIds = new Set<string>();
  for (const part of parts) {
    for (const id of part.questionIds) questionIds.add(id);
    for (const id of part.poolIds) poolIds.add(id);
  }
  return { questionIds: [...questionIds], poolIds: [...poolIds] };
}

/** Record heading and the labeled public identifier a person must copy. */
export function labeledQuestionRecognition(
  title: string | undefined,
  questionId: string,
  revision: number | string,
): { readonly title: string; readonly identifier: string } {
  return {
    title: recognitionTitle(title, "Question"),
    identifier: `Question ID ${questionId}, Revision ${revision}`,
  };
}

/** Pool heading and the labeled public identifier a person must copy. */
export function labeledPoolRecognition(
  title: string | undefined,
  poolId: string,
  edit: number | string,
): { readonly title: string; readonly identifier: string } {
  return {
    title: recognitionTitle(title, "Question Pool"),
    identifier: `Question Pool ID ${poolId}, Edit ${edit}`,
  };
}

/** One readable list of titles with their labeled public IDs. */
export function recognizedIdList(
  ids: readonly string[],
  titles: ReadonlyMap<string, string>,
  noun: string,
  idLabel: string,
): string {
  if (ids.length === 0) return "None";
  return ids.map((id) => `${recognitionTitle(titles.get(id), noun)} (${idLabel} ${id})`).join("; ");
}
