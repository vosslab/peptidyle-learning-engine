// Strict browser decoding for canonical Blueprint Course pair comparisons.

import { MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS } from "../../../generated/api/MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS";
import { MAX_ASSESSMENT_ORDERED_ENTRIES } from "../../../generated/api/MAX_ASSESSMENT_ORDERED_ENTRIES";
import type { BlueprintComparisonSide } from "../../../generated/api/BlueprintComparisonSide";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import type { CanonicalBlueprintCourse } from "../../../generated/api/CanonicalBlueprintCourse";
import {
  DecodeError,
  decodePositiveInteger,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeQuestionAttemptLimit, decodeQuestionAttemptTimeLimit } from "./question_model";
import { decodeCourseClassification } from "./course_classification";
import { decodeBoundedArray, field, requireOnlyFields } from "./shared";
import {
  assessmentType,
  blueprintEditNumber,
  defaults,
  pointValue,
  questionId,
  blueprintRevisionTuple,
  selectionRule,
  text,
} from "./blueprint_course";

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

function stableId(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!UUID.test(decoded)) throw new DecodeError(path, "a canonical UUID");
  return decoded;
}

function position(value: unknown, path: string): number {
  const decoded = decodeSafeInteger(value, path);
  if (decoded < 0 || decoded >= MAX_ASSESSMENT_ORDERED_ENTRIES)
    throw new DecodeError(path, "a bounded authored position");
  return decoded;
}

function canonicalAssessment(value: unknown, path: string): void {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assessment_type",
    "title",
    "instructions",
    "entries",
    "defaults",
  ]);
  assessmentType(field(record, "assessment_type", path), `${path}.assessment_type`);
  text(field(record, "title", path), `${path}.title`);
  const instructions = decodeString(field(record, "instructions", path), `${path}.instructions`);
  if (Array.from(instructions).length > MAX_ASSESSMENT_INSTRUCTIONS_UNICODE_SCALARS)
    throw new DecodeError(`${path}.instructions`, "bounded instructions");
  defaults(field(record, "defaults", path), `${path}.defaults`);
  const entries = decodeBoundedArray(
    field(record, "entries", path),
    `${path}.entries`,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    (entryValue, entryPath) => {
      const entry = decodeRecord(entryValue, entryPath);
      const kind = decodeStringEnum(field(entry, "kind", entryPath), `${entryPath}.kind`, [
        "fixed",
        "pool",
      ]);
      requireOnlyFields(
        entry,
        entryPath,
        kind === "fixed"
          ? [
              "kind",
              "published_question_revision_tuple",
              "points_possible",
              "scoring_rule",
              "question_attempt_limit",
              "question_attempt_time_limit",
            ]
          : [
              "kind",
              "question_pool_id",
              "question_pool_edit_number",
              "selection_count",
              "points_per_item",
              "scoring_rule",
              "selection_rule",
              "question_attempt_limit",
              "question_attempt_time_limit",
            ],
      );
      if (kind === "fixed") {
        const pinPath = `${entryPath}.published_question_revision_tuple`;
        const pin = decodeRecord(
          field(entry, "published_question_revision_tuple", entryPath),
          pinPath,
        );
        requireOnlyFields(pin, pinPath, ["publishedQuestionId", "revisionNumber"]);
        questionId(field(pin, "publishedQuestionId", pinPath), `${pinPath}.publishedQuestionId`);
        decodePositiveInteger(field(pin, "revisionNumber", pinPath), `${pinPath}.revisionNumber`);
        pointValue(field(entry, "points_possible", entryPath), `${entryPath}.points_possible`);
      } else {
        questionId(field(entry, "question_pool_id", entryPath), `${entryPath}.question_pool_id`);
        decodePositiveInteger(
          field(entry, "question_pool_edit_number", entryPath),
          `${entryPath}.question_pool_edit_number`,
        );
        decodePositiveInteger(
          field(entry, "selection_count", entryPath),
          `${entryPath}.selection_count`,
        );
        pointValue(field(entry, "points_per_item", entryPath), `${entryPath}.points_per_item`);
        selectionRule(field(entry, "selection_rule", entryPath), `${entryPath}.selection_rule`);
      }
      decodeStringEnum(field(entry, "scoring_rule", entryPath), `${entryPath}.scoring_rule`, [
        "normal",
        "fullCredit",
        "extraCredit",
        "excluded",
      ]);
      decodeQuestionAttemptLimit(
        field(entry, "question_attempt_limit", entryPath),
        `${entryPath}.question_attempt_limit`,
        true,
      );
      decodeQuestionAttemptTimeLimit(
        field(entry, "question_attempt_time_limit", entryPath),
        `${entryPath}.question_attempt_time_limit`,
        true,
      );
      return entryValue;
    },
  );
  if (entries.length === 0) throw new DecodeError(`${path}.entries`, "at least one entry");
}

/** Decodes the canonical accepted-result projection without inventing a second assessment parser. */
export function decodeCanonicalBlueprintCourse(
  value: unknown,
  path = "response",
): CanonicalBlueprintCourse {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["metadata", "modules"]);
  const metadataPath = `${path}.metadata`;
  const metadata = decodeRecord(field(record, "metadata", path), metadataPath);
  requireOnlyFields(metadata, metadataPath, ["short_name", "long_name", "classification"]);
  text(field(metadata, "short_name", metadataPath), `${metadataPath}.short_name`);
  text(field(metadata, "long_name", metadataPath), `${metadataPath}.long_name`);
  const classification = decodeCourseClassification(
    field(metadata, "classification", metadataPath),
    `${metadataPath}.classification`,
  );
  const modules = decodeBoundedArray(
    field(record, "modules", path),
    `${path}.modules`,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    (moduleValue, modulePath) => {
      const module = decodeRecord(moduleValue, modulePath);
      requireOnlyFields(module, modulePath, ["label", "assessments"]);
      const label = text(field(module, "label", modulePath), `${modulePath}.label`);
      const assessments = decodeBoundedArray(
        field(module, "assessments", modulePath),
        `${modulePath}.assessments`,
        MAX_ASSESSMENT_ORDERED_ENTRIES,
        (assessmentValue, assessmentPath) => {
          canonicalAssessment(assessmentValue, assessmentPath);
          return assessmentValue as CanonicalBlueprintCourse["modules"][number]["assessments"][number];
        },
      );
      return { label, assessments };
    },
  );
  return {
    metadata: {
      short_name: text(field(metadata, "short_name", metadataPath), `${metadataPath}.short_name`),
      long_name: text(field(metadata, "long_name", metadataPath), `${metadataPath}.long_name`),
      classification,
    },
    modules,
  };
}

function questionIds(input: unknown, path: string): string[] {
  const ids = decodeBoundedArray(
    input,
    path,
    MAX_ASSESSMENT_ORDERED_ENTRIES * MAX_ASSESSMENT_ORDERED_ENTRIES,
    questionId,
  );
  if (new Set(ids).size !== ids.length) throw new DecodeError(path, "unique Question IDs");
  return ids;
}

function side(input: unknown, path: string): BlueprintComparisonSide {
  const row = decodeRecord(input, path);
  requireOnlyFields(row, path, [
    "currentRevisionTuple",
    "names",
    "blueprintEditNumber",
    "modules",
    "assessments",
  ]);
  blueprintRevisionTuple(field(row, "currentRevisionTuple", path), `${path}.currentRevisionTuple`);
  const namesPath = `${path}.names`;
  const names = decodeRecord(field(row, "names", path), namesPath);
  requireOnlyFields(names, namesPath, ["shortName", "longName"]);
  text(field(names, "shortName", namesPath), `${namesPath}.shortName`);
  text(field(names, "longName", namesPath), `${namesPath}.longName`);
  blueprintEditNumber(field(row, "blueprintEditNumber", path), `${path}.blueprintEditNumber`);
  const modules = new Set<string>();
  decodeBoundedArray(
    field(row, "modules", path),
    `${path}.modules`,
    MAX_ASSESSMENT_ORDERED_ENTRIES,
    (moduleValue, modulePath) => {
      const module = decodeRecord(moduleValue, modulePath);
      requireOnlyFields(module, modulePath, ["blueprintModuleId", "position", "label"]);
      const moduleId = stableId(
        field(module, "blueprintModuleId", modulePath),
        `${modulePath}.blueprintModuleId`,
      );
      if (modules.has(moduleId)) throw new DecodeError(modulePath, "unique side-local Module ID");
      if (
        position(field(module, "position", modulePath), `${modulePath}.position`) !== modules.size
      )
        throw new DecodeError(modulePath, "Modules in contiguous authored order");
      modules.add(moduleId);
      text(field(module, "label", modulePath), `${modulePath}.label`);
      return moduleValue;
    },
  );
  const assessments = new Set<string>();
  const modulePositions = new Map<string, number>();
  decodeBoundedArray(
    field(row, "assessments", path),
    `${path}.assessments`,
    MAX_ASSESSMENT_ORDERED_ENTRIES * MAX_ASSESSMENT_ORDERED_ENTRIES,
    (assessmentValue, assessmentPath) => {
      const assessment = decodeRecord(assessmentValue, assessmentPath);
      requireOnlyFields(assessment, assessmentPath, [
        "blueprintAssessmentId",
        "blueprintModuleId",
        "position",
        "content",
        "questionIds",
      ]);
      const assessmentId = stableId(
        field(assessment, "blueprintAssessmentId", assessmentPath),
        `${assessmentPath}.blueprintAssessmentId`,
      );
      if (assessments.has(assessmentId))
        throw new DecodeError(assessmentPath, "unique side-local Assessment ID");
      assessments.add(assessmentId);
      const module = stableId(
        field(assessment, "blueprintModuleId", assessmentPath),
        `${assessmentPath}.blueprintModuleId`,
      );
      if (!modules.has(module))
        throw new DecodeError(assessmentPath, "an existing side-local Module");
      const authoredPosition = position(
        field(assessment, "position", assessmentPath),
        `${assessmentPath}.position`,
      );
      if (authoredPosition !== (modulePositions.get(module) ?? 0))
        throw new DecodeError(assessmentPath, "Assessments in contiguous Module order");
      modulePositions.set(module, authoredPosition + 1);
      canonicalAssessment(
        field(assessment, "content", assessmentPath),
        `${assessmentPath}.content`,
      );
      questionIds(
        field(assessment, "questionIds", assessmentPath),
        `${assessmentPath}.questionIds`,
      );
      return assessmentValue;
    },
  );
  return input as BlueprintComparisonSide;
}

function equalIds(actual: readonly string[], expected: readonly string[], path: string): void {
  const expectedIds = new Set(expected);
  if (actual.length !== expectedIds.size || actual.some((id) => !expectedIds.has(id)))
    throw new DecodeError(path, "the complete shared Question ID relationship");
}

// ASVS 1.5.2, 2.2.1: only complete canonical snapshots, with no metadata enrichment.
export function decodeBlueprintComparisonView(
  value: unknown,
  path = "response",
): BlueprintComparisonView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "left",
    "right",
    "assessmentRelationships",
    "sharedQuestionIds",
    "leftOnlyQuestionIds",
    "rightOnlyQuestionIds",
  ]);
  const left = side(field(record, "left", path), `${path}.left`);
  const right = side(field(record, "right", path), `${path}.right`);
  if (left.currentRevisionTuple.blueprintCourseId === right.currentRevisionTuple.blueprintCourseId)
    throw new DecodeError(path, "distinct compared Blueprint Courses");
  const leftIds = new Set(left.assessments.flatMap((assessment) => assessment.questionIds));
  const rightIds = new Set(right.assessments.flatMap((assessment) => assessment.questionIds));
  for (const [name, expected] of [
    ["sharedQuestionIds", [...leftIds].filter((id) => rightIds.has(id))],
    ["leftOnlyQuestionIds", [...leftIds].filter((id) => !rightIds.has(id))],
    ["rightOnlyQuestionIds", [...rightIds].filter((id) => !leftIds.has(id))],
  ] as const)
    equalIds(
      questionIds(field(record, name, path), `${path}.${name}`),
      expected,
      `${path}.${name}`,
    );
  const edges = new Set<string>();
  const leftAssessments = new Map(
    left.assessments.map((assessment) => [assessment.blueprintAssessmentId, assessment]),
  );
  const rightAssessments = new Map(
    right.assessments.map((assessment) => [assessment.blueprintAssessmentId, assessment]),
  );
  decodeBoundedArray(
    field(record, "assessmentRelationships", path),
    `${path}.assessmentRelationships`,
    MAX_ASSESSMENT_ORDERED_ENTRIES ** 4,
    (edgeValue, edgePath) => {
      const edge = decodeRecord(edgeValue, edgePath);
      requireOnlyFields(edge, edgePath, [
        "leftAssessmentId",
        "rightAssessmentId",
        "sharedQuestionIds",
      ]);
      const leftAssessmentId = stableId(
        field(edge, "leftAssessmentId", edgePath),
        `${edgePath}.leftAssessmentId`,
      );
      const rightAssessmentId = stableId(
        field(edge, "rightAssessmentId", edgePath),
        `${edgePath}.rightAssessmentId`,
      );
      const leftAssessment = leftAssessments.get(leftAssessmentId);
      const rightAssessment = rightAssessments.get(rightAssessmentId);
      if (!leftAssessment || !rightAssessment)
        throw new DecodeError(edgePath, "Assessment IDs belonging to the indicated sides");
      const key = `${leftAssessmentId}:${rightAssessmentId}`;
      if (edges.has(key)) throw new DecodeError(edgePath, "a unique relationship");
      edges.add(key);
      const rightQuestionIds = new Set(rightAssessment.questionIds);
      const shared = leftAssessment.questionIds.filter((id) => rightQuestionIds.has(id));
      if (shared.length === 0)
        throw new DecodeError(edgePath, "a nonempty shared Question ID relationship");
      equalIds(
        questionIds(field(edge, "sharedQuestionIds", edgePath), `${edgePath}.sharedQuestionIds`),
        shared,
        edgePath,
      );
      return edgeValue;
    },
  );
  return value as BlueprintComparisonView;
}
