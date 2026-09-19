// Strict decoding for participant-only frozen Blueprint Change Proposal records.

import type { BlueprintChangeProposalAcceptanceRequest } from "../../../generated/api/BlueprintChangeProposalAcceptanceRequest";
import type { BlueprintChangeProposalAcceptedView } from "../../../generated/api/BlueprintChangeProposalAcceptedView";
import type { BlueprintChangeProposalComparisonView } from "../../../generated/api/BlueprintChangeProposalComparisonView";
import type { BlueprintChangeProposalCreateRequest } from "../../../generated/api/BlueprintChangeProposalCreateRequest";
import type { BlueprintChangeProposalDecisionView } from "../../../generated/api/BlueprintChangeProposalDecisionView";
import type { BlueprintChangeProposalDetailView } from "../../../generated/api/BlueprintChangeProposalDetailView";
import type { BlueprintChangeProposalPageView } from "../../../generated/api/BlueprintChangeProposalPageView";
import type { BlueprintChangeProposalSideView } from "../../../generated/api/BlueprintChangeProposalSideView";
import type { BlueprintChangeProposalSummaryView } from "../../../generated/api/BlueprintChangeProposalSummaryView";
import type { BlueprintForkApplySelection } from "../../../generated/api/BlueprintForkApplySelection";
import {
  DecodeError,
  decodeBoolean,
  decodeDictionary,
  decodeNullable,
  decodeRecord,
  decodeUuid,
} from "../decoder";
import {
  decodeCanonicalBlueprintCourse,
  decodeBlueprintComparisonView,
} from "./blueprint_comparison";
import { blueprintEditNumber, blueprintRevisionTuple, text } from "./blueprint_course";
import { decodeCourseClassification } from "./course_classification";
import { decodeCursorPage, decodeTimestamp, field, requireOnlyFields } from "./shared";

function names(value: unknown, path: string): { shortName: string; longName: string } {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["shortName", "longName"]);
  return {
    shortName: text(field(record, "shortName", path), `${path}.shortName`),
    longName: text(field(record, "longName", path), `${path}.longName`),
  };
}

function summary(value: unknown, path: string): BlueprintChangeProposalSummaryView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "proposalId",
    "createdAt",
    "source",
    "sourceBlueprintEditNumber",
    "sourceNames",
    "target",
    "targetBlueprintEditNumber",
    "targetNames",
    "targetIsStale",
    "accepted",
  ]);
  const accepted = decodeNullable(
    field(record, "accepted", path),
    `${path}.accepted`,
    (item, itemPath) => {
      const row = decodeRecord(item, itemPath);
      requireOnlyFields(row, itemPath, ["acceptedAt", "target", "targetBlueprintEditNumber"]);
      return {
        acceptedAt: decodeTimestamp(field(row, "acceptedAt", itemPath), `${itemPath}.acceptedAt`),
        target: blueprintRevisionTuple(field(row, "target", itemPath), `${itemPath}.target`),
        targetBlueprintEditNumber: blueprintEditNumber(
          field(row, "targetBlueprintEditNumber", itemPath),
          `${itemPath}.targetBlueprintEditNumber`,
        ),
      };
    },
  );
  return {
    proposalId: decodeUuid(field(record, "proposalId", path), `${path}.proposalId`),
    createdAt: decodeTimestamp(field(record, "createdAt", path), `${path}.createdAt`),
    source: blueprintRevisionTuple(field(record, "source", path), `${path}.source`),
    sourceBlueprintEditNumber: blueprintEditNumber(
      field(record, "sourceBlueprintEditNumber", path),
      `${path}.sourceBlueprintEditNumber`,
    ),
    sourceNames: names(field(record, "sourceNames", path), `${path}.sourceNames`),
    target: blueprintRevisionTuple(field(record, "target", path), `${path}.target`),
    targetBlueprintEditNumber: blueprintEditNumber(
      field(record, "targetBlueprintEditNumber", path),
      `${path}.targetBlueprintEditNumber`,
    ),
    targetNames: names(field(record, "targetNames", path), `${path}.targetNames`),
    targetIsStale: decodeBoolean(field(record, "targetIsStale", path), `${path}.targetIsStale`),
    accepted,
  };
}

function selection(value: unknown, path: string): BlueprintForkApplySelection {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["sourceModuleLabels", "sourceAssessments", "layout"]);
  const copies = (
    name: "sourceModuleLabels" | "sourceAssessments",
    source: string,
    target: string,
  ): Array<Record<string, string | null>> => {
    const items = field(record, name, path);
    if (!Array.isArray(items)) throw new DecodeError(`${path}.${name}`, "an array");
    return items.map((item, index) => {
      const itemPath = `${path}.${name}[${index}]`;
      const row = decodeRecord(item, itemPath);
      requireOnlyFields(row, itemPath, [source, target]);
      return {
        [source]: decodeUuid(field(row, source, itemPath), `${itemPath}.${source}`),
        [target]: decodeNullable(field(row, target, itemPath), `${itemPath}.${target}`, decodeUuid),
      };
    });
  };
  const sourceModuleLabels = copies("sourceModuleLabels", "sourceModuleId", "targetModuleId");
  const sourceAssessments = copies("sourceAssessments", "sourceAssessmentId", "targetAssessmentId");
  const layout = decodeNullable(
    field(record, "layout", path),
    `${path}.layout`,
    (value, layoutPath) => {
      if (!Array.isArray(value) || value.length === 0)
        throw new DecodeError(layoutPath, "a nonempty complete layout");
      return value.map((rowValue, index) => {
        const rowPath = `${layoutPath}[${index}]`;
        const row = decodeRecord(rowValue, rowPath);
        requireOnlyFields(row, rowPath, ["module", "assessments"]);
        const destination = (
          input: unknown,
          destinationPath: string,
          module: boolean,
        ): Record<string, string> => {
          const destination = decodeRecord(input, destinationPath);
          const kind = field(destination, "kind", destinationPath);
          const key =
            kind === "existing"
              ? module
                ? "targetModuleId"
                : "targetAssessmentId"
              : kind === "newFromSource"
                ? module
                  ? "sourceModuleId"
                  : "sourceAssessmentId"
                : null;
          if (key === null) throw new DecodeError(destinationPath, "an explicit destination kind");
          requireOnlyFields(destination, destinationPath, ["kind", key]);
          return {
            kind: String(kind),
            [key]: decodeUuid(
              field(destination, key, destinationPath),
              `${destinationPath}.${key}`,
            ),
          };
        };
        const assessmentsValue = field(row, "assessments", rowPath);
        if (!Array.isArray(assessmentsValue) || assessmentsValue.length === 0)
          throw new DecodeError(`${rowPath}.assessments`, "a nonempty array");
        return {
          module: destination(field(row, "module", rowPath), `${rowPath}.module`, true),
          assessments: assessmentsValue.map((entry, entryIndex) =>
            destination(entry, `${rowPath}.assessments[${entryIndex}]`, false),
          ),
        };
      });
    },
  );
  return {
    sourceModuleLabels: sourceModuleLabels as BlueprintForkApplySelection["sourceModuleLabels"],
    sourceAssessments: sourceAssessments as BlueprintForkApplySelection["sourceAssessments"],
    layout: layout as BlueprintForkApplySelection["layout"],
  };
}

function decision(value: unknown, path: string): BlueprintChangeProposalDecisionView {
  const record = decodeRecord(value, path);
  const kind = field(record, "kind", path);
  if (kind === "entire") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind };
  }
  if (kind !== "selected") throw new DecodeError(`${path}.kind`, "entire or selected");
  requireOnlyFields(record, path, [
    "kind",
    "selection",
    "sourceShortName",
    "sourceLongName",
    "sourceClassification",
  ]);
  return {
    kind,
    selection: selection(field(record, "selection", path), `${path}.selection`),
    sourceShortName: decodeBoolean(
      field(record, "sourceShortName", path),
      `${path}.sourceShortName`,
    ),
    sourceLongName: decodeBoolean(field(record, "sourceLongName", path), `${path}.sourceLongName`),
    sourceClassification: decodeBoolean(
      field(record, "sourceClassification", path),
      `${path}.sourceClassification`,
    ),
  };
}

function side(
  value: unknown,
  path: string,
): Omit<BlueprintChangeProposalSideView, "modules" | "assessments"> & {
  modules: unknown;
  assessments: unknown;
} {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "revision",
    "blueprintEditNumber",
    "names",
    "classification",
    "modules",
    "assessments",
  ]);
  return {
    revision: blueprintRevisionTuple(field(record, "revision", path), `${path}.revision`),
    blueprintEditNumber: blueprintEditNumber(
      field(record, "blueprintEditNumber", path),
      `${path}.blueprintEditNumber`,
    ),
    names: names(field(record, "names", path), `${path}.names`),
    classification: decodeCourseClassification(
      field(record, "classification", path),
      `${path}.classification`,
    ),
    modules: field(record, "modules", path),
    assessments: field(record, "assessments", path),
  };
}

function comparison(value: unknown, path: string): BlueprintChangeProposalComparisonView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "source",
    "target",
    "assessmentRelationships",
    "sharedQuestionIds",
    "sourceOnlyQuestionIds",
    "targetOnlyQuestionIds",
  ]);
  const source = side(field(record, "source", path), `${path}.source`);
  const target = side(field(record, "target", path), `${path}.target`);
  const checked = decodeBlueprintComparisonView(
    {
      left: {
        currentRevision: source.revision,
        names: source.names,
        blueprintEditNumber: source.blueprintEditNumber,
        modules: source.modules,
        assessments: source.assessments,
      },
      right: {
        currentRevision: target.revision,
        names: target.names,
        blueprintEditNumber: target.blueprintEditNumber,
        modules: target.modules,
        assessments: target.assessments,
      },
      assessmentRelationships: field(record, "assessmentRelationships", path),
      sharedQuestionIds: field(record, "sharedQuestionIds", path),
      leftOnlyQuestionIds: field(record, "sourceOnlyQuestionIds", path),
      rightOnlyQuestionIds: field(record, "targetOnlyQuestionIds", path),
    },
    path,
  );
  return {
    source: { ...source, modules: checked.left.modules, assessments: checked.left.assessments },
    target: { ...target, modules: checked.right.modules, assessments: checked.right.assessments },
    assessmentRelationships: checked.assessmentRelationships,
    sharedQuestionIds: checked.sharedQuestionIds,
    sourceOnlyQuestionIds: checked.leftOnlyQuestionIds,
    targetOnlyQuestionIds: checked.rightOnlyQuestionIds,
  };
}

function accepted(value: unknown, path: string): BlueprintChangeProposalAcceptedView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "acceptedAt",
    "target",
    "targetBlueprintEditNumber",
    "decision",
    "appliedSelection",
    "newModules",
    "newAssessments",
    "resultingJson",
  ]);
  return {
    acceptedAt: decodeTimestamp(field(record, "acceptedAt", path), `${path}.acceptedAt`),
    target: blueprintRevisionTuple(field(record, "target", path), `${path}.target`),
    targetBlueprintEditNumber: blueprintEditNumber(
      field(record, "targetBlueprintEditNumber", path),
      `${path}.targetBlueprintEditNumber`,
    ),
    decision: decision(field(record, "decision", path), `${path}.decision`),
    appliedSelection: selection(
      field(record, "appliedSelection", path),
      `${path}.appliedSelection`,
    ),
    newModules: decodeDictionary(
      field(record, "newModules", path),
      `${path}.newModules`,
      decodeUuid,
    ),
    newAssessments: decodeDictionary(
      field(record, "newAssessments", path),
      `${path}.newAssessments`,
      decodeUuid,
    ),
    resultingJson: decodeCanonicalBlueprintCourse(
      field(record, "resultingJson", path),
      `${path}.resultingJson`,
    ),
  };
}

export function decodeBlueprintChangeProposalCreateRequest(
  value: unknown,
  path = "request",
): BlueprintChangeProposalCreateRequest {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "source",
    "sourceBlueprintEditNumber",
    "target",
    "targetBlueprintEditNumber",
  ]);
  return {
    source: blueprintRevisionTuple(field(record, "source", path), `${path}.source`),
    sourceBlueprintEditNumber: blueprintEditNumber(
      field(record, "sourceBlueprintEditNumber", path),
      `${path}.sourceBlueprintEditNumber`,
    ),
    target: blueprintRevisionTuple(field(record, "target", path), `${path}.target`),
    targetBlueprintEditNumber: blueprintEditNumber(
      field(record, "targetBlueprintEditNumber", path),
      `${path}.targetBlueprintEditNumber`,
    ),
  };
}

export function decodeBlueprintChangeProposalAcceptanceRequest(
  value: unknown,
  path = "request",
): BlueprintChangeProposalAcceptanceRequest {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "expectedTarget",
    "expectedTargetBlueprintEditNumber",
    "decision",
  ]);
  return {
    expectedTarget: blueprintRevisionTuple(
      field(record, "expectedTarget", path),
      `${path}.expectedTarget`,
    ),
    expectedTargetBlueprintEditNumber: blueprintEditNumber(
      field(record, "expectedTargetBlueprintEditNumber", path),
      `${path}.expectedTargetBlueprintEditNumber`,
    ),
    decision: decision(field(record, "decision", path), `${path}.decision`),
  };
}

export function decodeBlueprintChangeProposalPageView(
  value: unknown,
  path = "response",
): BlueprintChangeProposalPageView {
  const page = decodeCursorPage(value, path, summary);
  return { items: [...page.items], nextCursor: page.nextCursor };
}

export function decodeBlueprintChangeProposalDetailView(
  value: unknown,
  path = "response",
): BlueprintChangeProposalDetailView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["proposal", "canAccept", "comparison", "accepted"]);
  return {
    proposal: summary(field(record, "proposal", path), `${path}.proposal`),
    canAccept: decodeBoolean(field(record, "canAccept", path), `${path}.canAccept`),
    comparison: comparison(field(record, "comparison", path), `${path}.comparison`),
    accepted: decodeNullable(field(record, "accepted", path), `${path}.accepted`, accepted),
  };
}

export function decodeBlueprintChangeProposalAcceptedView(
  value: unknown,
  path = "response",
): BlueprintChangeProposalAcceptedView {
  return accepted(value, path);
}
