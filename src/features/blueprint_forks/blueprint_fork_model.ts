// Read-only presentation of the server's complete canonical Revision comparison.

import type { BlueprintComparisonSide } from "../../../generated/api/BlueprintComparisonSide";

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

/** Module references are resolved only within their own Blueprint Course. */
export function forkModuleLabel(side: BlueprintComparisonSide, reference: string): string {
  return (
    side.modules.find((module) => module.blueprintModuleId === reference)?.label ?? reference
  );
}

export function readableSettingName(value: string): string {
  const words = value.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/_/g, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** Shared Questions relate Assessments; neither names nor local references imply identity. */
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
      leftSide.modules.find(
        (item) => item.blueprintModuleId === left.blueprintModuleId,
      )?.position,
      rightSide.modules.find(
        (item) => item.blueprintModuleId === right.blueprintModuleId,
      )?.position,
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
