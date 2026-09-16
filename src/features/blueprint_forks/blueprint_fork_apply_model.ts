import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import type { BlueprintForkApplyModuleLayout } from "../../../generated/api/BlueprintForkApplyModuleLayout";
import type { BlueprintForkApplySelection } from "../../../generated/api/BlueprintForkApplySelection";
import { MAX_REUSABLE_ENTRIES } from "../blueprint_course/blueprint_course_model";
export type Layout = BlueprintForkApplyModuleLayout;
export function destinationKey(value: Layout["module"] | Layout["assessments"][number]): string {
  return (
    value.kind +
    ":" +
    ("targetModuleReference" in value
      ? value.targetModuleReference
      : "sourceModuleReference" in value
        ? value.sourceModuleReference
        : "targetAssessmentReference" in value
          ? value.targetAssessmentReference
          : value.sourceAssessmentReference)
  );
}
export function currentForkLayout(review: BlueprintComparisonView): Layout[] {
  return [...review.right.modules]
    .sort((a, b) => a.position - b.position)
    .map((module) => ({
      module: { kind: "existing", targetModuleReference: module.blueprintModuleReference },
      assessments: review.right.assessments
        .filter((a) => a.blueprintModuleReference === module.blueprintModuleReference)
        .sort((a, b) => a.position - b.position)
        .map((a) => ({
          kind: "existing",
          targetAssessmentReference: a.blueprintAssessmentReference,
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
    new Set(labels.map((c) => c.sourceModuleReference)).size !== labels.length ||
    new Set(assessments.map((c) => c.sourceAssessmentReference)).size !== assessments.length
  )
    return "Copy each source only once.";
  if (
    new Set(
      labels.flatMap((c) => (c.targetModuleReference === null ? [] : [c.targetModuleReference])),
    ).size !== labels.filter((c) => c.targetModuleReference !== null).length ||
    new Set(
      assessments.flatMap((c) =>
        c.targetAssessmentReference === null ? [] : [c.targetAssessmentReference],
      ),
    ).size !== assessments.filter((c) => c.targetAssessmentReference !== null).length
  )
    return "Choose a different target for each source copy.";
  for (const copy of labels)
    if (
      !review.left.modules.some((m) => m.blueprintModuleReference === copy.sourceModuleReference) ||
      !modules.includes(
        destinationKey(
          copy.targetModuleReference === null
            ? { kind: "newFromSource", sourceModuleReference: copy.sourceModuleReference }
            : { kind: "existing", targetModuleReference: copy.targetModuleReference },
        ),
      )
    )
      return "Every selected source label needs a destination module.";
  for (const copy of assessments)
    if (
      !review.left.assessments.some(
        (a) => a.blueprintAssessmentReference === copy.sourceAssessmentReference,
      ) ||
      !entries.includes(
        destinationKey(
          copy.targetAssessmentReference === null
            ? { kind: "newFromSource", sourceAssessmentReference: copy.sourceAssessmentReference }
            : { kind: "existing", targetAssessmentReference: copy.targetAssessmentReference },
        ),
      )
    )
      return "Every selected source Assessment needs a destination.";
  for (const row of layout) {
    const module = row.module;
    if (
      module.kind === "existing"
        ? !review.right.modules.some(
            (m) => m.blueprintModuleReference === module.targetModuleReference,
          )
        : !labels.some(
            (c) =>
              c.sourceModuleReference === module.sourceModuleReference &&
              c.targetModuleReference === null,
          )
    )
      return "Choose a valid existing module or select the new module's source label.";
    for (const entry of row.assessments)
      if (
        entry.kind === "existing"
          ? !review.right.assessments.some(
              (a) => a.blueprintAssessmentReference === entry.targetAssessmentReference,
            )
          : !assessments.some(
              (c) =>
                c.sourceAssessmentReference === entry.sourceAssessmentReference &&
                c.targetAssessmentReference === null,
            )
      )
        return "Choose a valid existing Assessment or select complete content for the new copy.";
  }
  return null;
}
