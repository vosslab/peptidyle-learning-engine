// Read-only presentation of the server's complete canonical Revision comparison.

import type { BlueprintComparisonSide } from "../../../generated/api/BlueprintComparisonSide";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";

function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value !== null && typeof value === "object") {
    return `{${Object.entries(value)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([key, item]) => `${JSON.stringify(key)}:${canonical(item)}`)
      .join(",")}}`;
  }
  return JSON.stringify(value) ?? "null";
}

export function sameForkSnapshot(left: unknown, right: unknown): boolean {
  return canonical(left) === canonical(right);
}

/** Module IDs are resolved only within their own Blueprint Course. */
export function forkModuleLabel(side: BlueprintComparisonSide, blueprintModuleId: string): string {
  return (
    side.modules.find((module) => module.blueprintModuleId === blueprintModuleId)?.label ??
    blueprintModuleId
  );
}

export function readableSettingName(value: string): string {
  const words = value.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/_/g, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** Shared Questions relate Assessments; neither names nor local labels imply identity. */
export function assessmentDifferenceLabels(
  left: BlueprintComparisonSide["assessments"][number],
  right: BlueprintComparisonSide["assessments"][number],
  leftSide: BlueprintComparisonSide,
  rightSide: BlueprintComparisonSide,
): string {
  const fields: ReadonlyArray<readonly [string, unknown, unknown]> = [
    [
      "Module label",
      forkModuleLabel(leftSide, left.blueprintModuleId),
      forkModuleLabel(rightSide, right.blueprintModuleId),
    ],
    [
      "Module order",
      leftSide.modules.find((item) => item.blueprintModuleId === left.blueprintModuleId)?.position,
      rightSide.modules.find((item) => item.blueprintModuleId === right.blueprintModuleId)
        ?.position,
    ],
    ["Assessment order", left.position, right.position],
    ["Title", left.content.title, right.content.title],
    ["Assessment Type", left.content.assessment_type, right.content.assessment_type],
    ["Instructions", left.content.instructions, right.content.instructions],
    ["Questions, Pools, pins, order or scoring", left.content.entries, right.content.entries],
    ["Assessment Properties", left.content.defaults, right.content.defaults],
  ];
  const changed = fields.filter(([, a, b]) => !sameForkSnapshot(a, b)).map(([label]) => label);
  return changed.length > 0
    ? `Current differences: ${changed.join("; ")}.`
    : "Compared canonical fields and placement match.";
}

/** Four comparison outcomes for one kind of Blueprint content. */
export interface ComparisonMembership {
  readonly shared: readonly string[];
  readonly added: readonly string[];
  readonly removed: readonly string[];
  readonly changed: readonly string[];
}

function sortedIds(ids: Iterable<string>): string[] {
  return [...ids].sort((left, right) => left.localeCompare(right));
}

function questionIds(side: BlueprintComparisonSide): Set<string> {
  return new Set(side.assessments.flatMap((assessment) => assessment.questionIds));
}

function revisionPins(side: BlueprintComparisonSide): Map<string, string> {
  const grouped = new Map<string, string[]>();
  for (const assessment of side.assessments) {
    for (const entry of assessment.content.entries) {
      if (entry.kind !== "fixed") continue;
      const id = entry.published_question_revision_tuple.publishedQuestionId;
      const pins = grouped.get(id) ?? [];
      pins.push(String(entry.published_question_revision_tuple.revisionNumber));
      grouped.set(id, pins);
    }
  }
  return new Map([...grouped].map(([id, pins]) => [id, pins.sort().join(",")]));
}

/** Shared Question IDs come from each side's inventory. A changed row is a shared ID whose Revision pin differs. */
export function publishedQuestionComparison(view: BlueprintComparisonView): ComparisonMembership {
  const left = questionIds(view.left);
  const right = questionIds(view.right);
  const shared = sortedIds([...left].filter((id) => right.has(id)));
  const leftPins = revisionPins(view.left);
  const rightPins = revisionPins(view.right);
  return {
    shared,
    added: sortedIds([...right].filter((id) => !left.has(id))),
    removed: sortedIds([...left].filter((id) => !right.has(id))),
    changed: shared.filter((id) => {
      const leftPin = leftPins.get(id);
      const rightPin = rightPins.get(id);
      return leftPin !== undefined && rightPin !== undefined && leftPin !== rightPin;
    }),
  };
}

function poolSnapshots(side: BlueprintComparisonSide): Map<string, string> {
  const grouped = new Map<string, string[]>();
  for (const assessment of side.assessments) {
    for (const entry of assessment.content.entries) {
      if (entry.kind !== "pool") continue;
      const snapshots = grouped.get(entry.question_pool_id) ?? [];
      snapshots.push(canonical(entry));
      grouped.set(entry.question_pool_id, snapshots);
    }
  }
  return new Map([...grouped].map(([id, snapshots]) => [id, snapshots.sort().join("|")]));
}

function partitionedIds(
  left: Map<string, string>,
  right: Map<string, string>,
): ComparisonMembership {
  const shared: string[] = [];
  const changed: string[] = [];
  const removed: string[] = [];
  for (const [id, snapshot] of left) {
    const other = right.get(id);
    if (other === undefined) removed.push(id);
    else if (other === snapshot) shared.push(id);
    else changed.push(id);
  }
  return {
    shared: sortedIds(shared),
    added: sortedIds([...right.keys()].filter((id) => !left.has(id))),
    removed: sortedIds(removed),
    changed: sortedIds(changed),
  };
}

/** Question Pool identity is the Pool ID. The same ID with different canonical entry content is changed. */
export function questionPoolComparison(view: BlueprintComparisonView): ComparisonMembership {
  return partitionedIds(poolSnapshots(view.left), poolSnapshots(view.right));
}

function assessmentById(
  side: BlueprintComparisonSide,
  blueprintAssessmentId: string,
): BlueprintComparisonSide["assessments"][number] | undefined {
  return side.assessments.find((item) => item.blueprintAssessmentId === blueprintAssessmentId);
}

/** Related Assessments are shared when their canonical fields match and changed when those fields differ. */
export function assessmentComparison(view: BlueprintComparisonView): ComparisonMembership {
  const relatedLeft = new Set(view.assessmentRelationships.map((edge) => edge.leftAssessmentId));
  const relatedRight = new Set(view.assessmentRelationships.map((edge) => edge.rightAssessmentId));
  const shared: string[] = [];
  const changed: string[] = [];
  for (const edge of view.assessmentRelationships) {
    const left = assessmentById(view.left, edge.leftAssessmentId);
    const right = assessmentById(view.right, edge.rightAssessmentId);
    if (left === undefined || right === undefined) continue;
    const label = `${left.content.title} compared with ${right.content.title}`;
    const differences = assessmentDifferenceLabels(left, right, view.left, view.right);
    if (differences.startsWith("Compared canonical fields")) shared.push(label);
    else changed.push(`${label}: ${differences}`);
  }
  return {
    shared: sortedIds(shared),
    added: sortedIds(
      view.right.assessments
        .filter((item) => !relatedRight.has(item.blueprintAssessmentId))
        .map((item) => item.content.title),
    ),
    removed: sortedIds(
      view.left.assessments
        .filter((item) => !relatedLeft.has(item.blueprintAssessmentId))
        .map((item) => item.content.title),
    ),
    changed: sortedIds(changed),
  };
}

/** Small settings reader: retain every canonical property without a JSON-only presentation. */
export function settingLines(
  value: unknown,
  prefix = "",
): ReadonlyArray<{ label: string; value: string }> {
  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    return Object.entries(value).flatMap(([key, item]) =>
      settingLines(
        item,
        prefix ? `${prefix}: ${readableSettingName(key)}` : readableSettingName(key),
      ),
    );
  }
  return [
    {
      label: prefix,
      value:
        value === null
          ? /limit|attempts/i.test(prefix)
            ? "Unlimited"
            : "Not set"
          : typeof value === "boolean"
            ? value
              ? "Yes"
              : "No"
            : typeof value === "string"
              ? readableSettingName(value)
              : typeof value === "number"
                ? String(value)
                : (JSON.stringify(value) ?? "Not set"),
    },
  ];
}
