// Strict decoding for the selective-update command and atomic public result.

import type { BlueprintForkApplyRequest } from "../../../generated/api/BlueprintForkApplyRequest";
import type { BlueprintForkApplyResponse } from "../../../generated/api/BlueprintForkApplyResponse";
import { MAX_ASSESSMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSESSMENT_ORDERED_ENTRIES";
import { DecodeError, decodeBoolean, decodeNullable, decodeRecord, decodeUuid } from "../decoder";
import { decodeBoundedArray, field, requireOnlyFields } from "./shared";
import {
  blueprintEditNumber,
  decodeBlueprintMetadataState,
  revisionReference,
} from "./blueprint_course";

function copies(
  value: unknown,
  path: string,
  sourceField: string,
  targetField: string,
  maximum: number = MAX_ASSESSMENT_ORDERED_ENTRIES,
): Array<Record<string, string | null>> {
  const result = decodeBoundedArray(value, path, maximum, (value, path) => {
    const record = decodeRecord(value, path);
    requireOnlyFields(record, path, [sourceField, targetField]);
    return {
      [sourceField]: decodeUuid(field(record, sourceField, path), `${path}.${sourceField}`),
      [targetField]: decodeNullable(
        field(record, targetField, path),
        `${path}.${targetField}`,
        decodeUuid,
      ),
    };
  });
  const sources = result.map((r) => r[sourceField]),
    targets = result.map((r) => r[targetField]).filter((r) => r !== null);
  if (new Set(sources).size !== sources.length || new Set(targets).size !== targets.length)
    throw new DecodeError(path, "unique source and non-null target references");
  return result;
}

function destination(value: unknown, path: string, module: boolean): Record<string, string> {
  const record = decodeRecord(value, path);
  const kind = field(record, "kind", path);
  if (kind !== "existing" && kind !== "newFromSource")
    throw new DecodeError(path, "an explicit destination kind");
  const name =
    kind === "existing"
      ? module
        ? "targetModuleReference"
        : "targetAssessmentReference"
      : module
        ? "sourceModuleReference"
        : "sourceAssessmentReference";
  requireOnlyFields(record, path, ["kind", name]);
  return { kind, [name]: decodeUuid(field(record, name, path), `${path}.${name}`) };
}

/** ASVS 1.5.2/2.2.1: allow only pinned heads, metadata validators, and domain choices. */
export function decodeBlueprintForkApplyRequest(
  value: unknown,
  path = "request",
): BlueprintForkApplyRequest {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "expectedSource",
    "expectedFork",
    "expectedSourceBlueprintEditNumber",
    "expectedForkBlueprintEditNumber",
    "sourceShortName",
    "sourceLongName",
    "selection",
  ]);
  const selectionPath = `${path}.selection`;
  const selection = decodeRecord(field(record, "selection", path), selectionPath);
  requireOnlyFields(selection, selectionPath, [
    "sourceModuleLabels",
    "sourceAssessments",
    "layout",
  ]);
  const layout = decodeNullable(
    field(selection, "layout", selectionPath),
    `${selectionPath}.layout`,
    (input, inputPath) => {
      const rows = decodeBoundedArray(
        input,
        inputPath,
        MAX_ASSESSMENT_ORDERED_ENTRIES,
        (row, rowPath) => {
          const item = decodeRecord(row, rowPath);
          requireOnlyFields(item, rowPath, ["module", "assessments"]);
          return {
            module: destination(field(item, "module", rowPath), `${rowPath}.module`, true),
            assessments: decodeBoundedArray(
              field(item, "assessments", rowPath),
              `${rowPath}.assessments`,
              MAX_ASSESSMENT_ORDERED_ENTRIES,
              (v, p) => destination(v, p, false),
            ),
          };
        },
      );
      if (rows.length === 0) throw new DecodeError(inputPath, "a nonempty complete layout");
      if (rows.some((row) => row.assessments.length === 0))
        throw new DecodeError(inputPath, "nonempty modules");
      const modules = rows.map((row) => JSON.stringify(row.module));
      const assessments = rows.flatMap((row) => row.assessments.map((a) => JSON.stringify(a)));
      if (
        new Set(modules).size !== modules.length ||
        new Set(assessments).size !== assessments.length
      )
        throw new DecodeError(inputPath, "globally unique module and Assessment references");
      return rows;
    },
  );
  return {
    expectedSource: revisionReference(
      field(record, "expectedSource", path),
      `${path}.expectedSource`,
    ),
    expectedFork: revisionReference(field(record, "expectedFork", path), `${path}.expectedFork`),
    expectedSourceBlueprintEditNumber: blueprintEditNumber(
      field(record, "expectedSourceBlueprintEditNumber", path),
      `${path}.expectedSourceBlueprintEditNumber`,
    ),
    expectedForkBlueprintEditNumber: blueprintEditNumber(
      field(record, "expectedForkBlueprintEditNumber", path),
      `${path}.expectedForkBlueprintEditNumber`,
    ),
    sourceShortName: decodeBoolean(
      field(record, "sourceShortName", path),
      `${path}.sourceShortName`,
    ),
    sourceLongName: decodeBoolean(field(record, "sourceLongName", path), `${path}.sourceLongName`),
    selection: {
      sourceModuleLabels: copies(
        field(selection, "sourceModuleLabels", selectionPath),
        `${selectionPath}.sourceModuleLabels`,
        "sourceModuleReference",
        "targetModuleReference",
      ) as BlueprintForkApplyRequest["selection"]["sourceModuleLabels"],
      sourceAssessments: copies(
        field(selection, "sourceAssessments", selectionPath),
        `${selectionPath}.sourceAssessments`,
        "sourceAssessmentReference",
        "targetAssessmentReference",
        MAX_ASSESSMENT_ORDERED_ENTRIES * MAX_ASSESSMENT_ORDERED_ENTRIES,
      ) as BlueprintForkApplyRequest["selection"]["sourceAssessments"],
      layout: layout as BlueprintForkApplyRequest["selection"]["layout"],
    },
  };
}

/** ASVS 8.2.3: reject private receipt fields and arbitrary post-commit content. */
export function decodeBlueprintForkApplyResponse(
  value: unknown,
  path = "response",
): BlueprintForkApplyResponse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["blueprintRevision", "changed", "metadata"]);
  return {
    blueprintRevision: revisionReference(
      field(record, "blueprintRevision", path),
      `${path}.blueprintRevision`,
    ),
    changed: decodeBoolean(field(record, "changed", path), `${path}.changed`),
    metadata: decodeBlueprintMetadataState(field(record, "metadata", path), `${path}.metadata`),
  };
}
