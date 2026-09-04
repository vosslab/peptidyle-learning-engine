import type { CourseGradeSchemeUpdateView } from "../../generated/api/CourseGradeSchemeUpdateView";
import type { GradeCategoryReference } from "../../generated/api/GradeCategoryReference";

export function renumberAssignmentsByCategory(
  draft: CourseGradeSchemeUpdateView,
): CourseGradeSchemeUpdateView {
  const next = structuredClone(draft);
  for (const category of next.scheme.categories) {
    const members = next.assignments.filter((item) => item.category === category.id);
    members.sort((left, right) => (left.position ?? 0) - (right.position ?? 0));
    members.forEach((item, index) => {
      item.position = index;
    });
  }
  return next;
}

export function categoryAssignments(
  draft: CourseGradeSchemeUpdateView,
  category: GradeCategoryReference,
): number {
  return draft.assignments.filter((item) => item.category === category).length;
}

export function gradeSettingsErrors(draft: CourseGradeSchemeUpdateView): ReadonlyArray<string> {
  const errors: string[] = [];
  if (draft.scheme.mode === "weightedCategories") {
    const weight = draft.scheme.categories.reduce((sum, item) => sum + item.weightBasisPoints, 0);
    if (draft.scheme.categories.length === 0) errors.push("Add at least one weighted category.");
    if (weight !== 10_000) errors.push("Weighted categories must total exactly 100.00%.");
    if (draft.assignments.some((item) => item.included && item.category === null))
      errors.push("Every included assignment needs a category.");
    for (const category of draft.scheme.categories) {
      if (
        category.title.trim() !== category.title ||
        Array.from(category.title).length < 1 ||
        Array.from(category.title).length > 200
      )
        errors.push("Category titles must be trimmed and 1 to 200 characters.");
      if (category.weightBasisPoints < 1 || category.weightBasisPoints > 10_000)
        errors.push("Each category needs a positive weight.");
      const included = draft.assignments.filter(
        (item) => item.included && item.category === category.id,
      ).length;
      if (category.dropLowest >= included)
        errors.push("A category must retain at least one included assignment after drops.");
    }
  }
  if (
    draft.scheme.letterGradeBands.some(
      (item) =>
        item.label.trim() !== item.label ||
        Array.from(item.label).length < 1 ||
        Array.from(item.label).length > 32,
    )
  )
    errors.push("Letter-band labels must be trimmed and 1 to 32 characters.");
  if (
    new Set(draft.scheme.letterGradeBands.map((item) => item.label)).size !==
    draft.scheme.letterGradeBands.length
  )
    errors.push("Letter-band labels must be unique.");
  for (let index = 1; index < draft.scheme.letterGradeBands.length; index += 1) {
    if (
      draft.scheme.letterGradeBands[index - 1]!.minimumBasisPoints <=
      draft.scheme.letterGradeBands[index]!.minimumBasisPoints
    ) {
      errors.push("Letter-band thresholds must be in descending order.");
      break;
    }
  }
  return errors;
}

export function percentToBasisPoints(value: string): number | undefined {
  if (!/^\d{1,3}(?:\.\d{1,2})?$/u.test(value)) return undefined;
  const points = Math.round(Number(value) * 100);
  return points >= 0 && points <= 10_000 ? points : undefined;
}
