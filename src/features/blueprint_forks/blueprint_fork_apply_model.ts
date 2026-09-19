import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import type { BlueprintForkApplyModuleLayout } from "../../../generated/api/BlueprintForkApplyModuleLayout";
import type { BlueprintForkApplySelection } from "../../../generated/api/BlueprintForkApplySelection";
import { MAX_REUSABLE_ENTRIES } from "../blueprint_course/blueprint_course_model";
export type Layout = BlueprintForkApplyModuleLayout;
export function destinationKey(value: Layout["module"] | Layout["assessments"][number]): string {
  return (
    value.kind +
    ":" +
    ("targetModuleId" in value
      ? value.targetModuleId
      : "sourceModuleId" in value
        ? value.sourceModuleId
        : "targetAssessmentId" in value
          ? value.targetAssessmentId
          : value.sourceAssessmentId)
  );
}
export function currentForkLayout(review: BlueprintComparisonView): Layout[] {
  return [...review.right.modules]
    .sort((a, b) => a.position - b.position)
    .map((module) => ({
      module: { kind: "existing", targetModuleId: module.blueprintModuleId },
      assessments: review.right.assessments
        .filter((a) => a.blueprintModuleId === module.blueprintModuleId)
        .sort((a, b) => a.position - b.position)
        .map((a) => ({
          kind: "existing",
          targetAssessmentId: a.blueprintAssessmentId,
        })),
    }));
}
export function reordered<T>(items: readonly T[], position: number, offset: number): T[] {
  const result = [...items];
  const destination = position + offset;
  if (destination < 0 || destination >= result.length) return result;
  const item = result[position];
  if (item === undefined) return result;
  result.splice(position, 1);
  result.splice(destination, 0, item);
  return result;
}
export function forkSelectionProblem(
  review: BlueprintComparisonView,
  layout: readonly Layout[],
  labels: BlueprintForkApplySelection["sourceModuleLabels"],
  assessments: BlueprintForkApplySelection["sourceAssessments"],
): string | null {
  if (!layout.length || layout.length > MAX_REUSABLE_ENTRIES)
    return `Use between 1 and ${MAX_REUSABLE_ENTRIES} destination modules.`;
  if (layout.some((m) => !m.assessments.length || m.assessments.length > MAX_REUSABLE_ENTRIES))
    return `Each module needs between 1 and ${MAX_REUSABLE_ENTRIES} Assessments.`;
  const modules = layout.map((m) => destinationKey(m.module));
  const entries = layout.flatMap((m) => m.assessments.map(destinationKey));
  if (new Set(modules).size !== modules.length || new Set(entries).size !== entries.length)
    return "Each destination may appear only once.";
  if (
    new Set(labels.map((c) => c.sourceModuleId)).size !== labels.length ||
    new Set(assessments.map((c) => c.sourceAssessmentId)).size !== assessments.length
  )
    return "Copy each source only once.";
  if (
    new Set(
      labels.flatMap((c) => (c.targetModuleId === null ? [] : [c.targetModuleId])),
    ).size !== labels.filter((c) => c.targetModuleId !== null).length ||
    new Set(
      assessments.flatMap((c) => (c.targetAssessmentId === null ? [] : [c.targetAssessmentId])),
    ).size !== assessments.filter((c) => c.targetAssessmentId !== null).length
  )
    return "Choose a different target for each source copy.";
  for (const copy of labels)
    if (
      !review.left.modules.some((m) => m.blueprintModuleId === copy.sourceModuleId) ||
      !modules.includes(
        destinationKey(
          copy.targetModuleId === null
            ? { kind: "newFromSource", sourceModuleId: copy.sourceModuleId }
            : { kind: "existing", targetModuleId: copy.targetModuleId },
        ),
      )
    )
      return "Every selected source label needs a destination module.";
  for (const copy of assessments)
    if (
      !review.left.assessments.some((a) => a.blueprintAssessmentId === copy.sourceAssessmentId) ||
      !entries.includes(
        destinationKey(
          copy.targetAssessmentId === null
            ? { kind: "newFromSource", sourceAssessmentId: copy.sourceAssessmentId }
            : { kind: "existing", targetAssessmentId: copy.targetAssessmentId },
        ),
      )
    )
      return "Every selected source Assessment needs a destination.";
  for (const row of layout) {
    const module = row.module;
    if (
      module.kind === "existing"
        ? !review.right.modules.some(
            (m) => m.blueprintModuleId === module.targetModuleId,
          )
        : !labels.some(
            (c) =>
              c.sourceModuleId === module.sourceModuleId &&
              c.targetModuleId === null,
          )
    )
      return "Choose a valid existing module or select the new module's source label.";
    for (const entry of row.assessments)
      if (
        entry.kind === "existing"
          ? !review.right.assessments.some(
              (a) => a.blueprintAssessmentId === entry.targetAssessmentId,
            )
          : !assessments.some(
              (c) =>
                c.sourceAssessmentId === entry.sourceAssessmentId && c.targetAssessmentId === null,
            )
      )
        return "Choose a valid existing Assessment or select complete content for the new copy.";
  }
  return null;
}
