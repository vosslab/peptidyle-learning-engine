// Closed, bounded decoders for the answer-free B2 curriculum-adoption API.

import { MAX_ASSIGNMENT_ORDERED_ENTRIES } from "../../../../generated/api/MAX_ASSIGNMENT_ORDERED_ENTRIES";
import { MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP } from "../../../../generated/api/MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP";
import { MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES } from "../../../../generated/api/MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES";
import { MAX_CURRICULUM_ADOPTION_IDEMPOTENCY_KEY_BYTES } from "../../../../generated/api/MAX_CURRICULUM_ADOPTION_IDEMPOTENCY_KEY_BYTES";
import type { AssignmentDefinitionSourceView } from "../../../../generated/api/AssignmentDefinitionSourceView";
import type { AssignmentFastForwardDecision } from "../../../../generated/api/AssignmentFastForwardDecision";
import type { AssignmentFastForwardPreviewView } from "../../../../generated/api/AssignmentFastForwardPreviewView";
import type { UnavailablePinRecoveryAction } from "../../../../generated/api/UnavailablePinRecoveryAction";
import type { PreservedAssignmentRecoveryAction } from "../../../../generated/api/PreservedAssignmentRecoveryAction";
import type { AlphaInstantiationPreviewRequest } from "../../../../generated/api/AlphaInstantiationPreviewRequest";
import type { AlphaInstantiationPreviewView } from "../../../../generated/api/AlphaInstantiationPreviewView";
import type { BlueprintInstantiationPreviewRequest } from "../../../../generated/api/BlueprintInstantiationPreviewRequest";
import type { BlueprintInstantiationPreviewView } from "../../../../generated/api/BlueprintInstantiationPreviewView";
import type { CourseRolloverPreviewRequest } from "../../../../generated/api/CourseRolloverPreviewRequest";
import type { CourseRolloverPreviewView } from "../../../../generated/api/CourseRolloverPreviewView";
import type { CourseTermShiftPreviewOutcome } from "../../../../generated/api/CourseTermShiftPreviewOutcome";
import type { CourseTermShiftPreviewRequest } from "../../../../generated/api/CourseTermShiftPreviewRequest";
import type { CourseTermShiftPreviewView } from "../../../../generated/api/CourseTermShiftPreviewView";
import type { CourseScheduleWitness } from "../../../../generated/api/CourseScheduleWitness";
import type { CurriculumAdoptionReconciliationResult } from "../../../../generated/api/CurriculumAdoptionReconciliationResult";
import type { CurriculumAdoptionRepairedProjection } from "../../../../generated/api/CurriculumAdoptionRepairedProjection";
import type { CurriculumAdoptionReceiptBinding } from "../../../../generated/api/CurriculumAdoptionReceiptBinding";
import type { CurriculumAssignmentImportSourceView } from "../../../../generated/api/CurriculumAssignmentImportSourceView";
import type { CurriculumAssignmentView } from "../../../../generated/api/CurriculumAssignmentView";
import type { CurriculumCourseImportOriginView } from "../../../../generated/api/CurriculumCourseImportOriginView";
import type { CurriculumCourseImportView } from "../../../../generated/api/CurriculumCourseImportView";
import type { CurriculumImportView } from "../../../../generated/api/CurriculumImportView";
import type { CurriculumPinPosition } from "../../../../generated/api/CurriculumPinPosition";
import type { CurriculumPinReplacement } from "../../../../generated/api/CurriculumPinReplacement";
import type { CurriculumScheduleCorrection } from "../../../../generated/api/CurriculumScheduleCorrection";
import type { ForkAlphaPreviewRequest } from "../../../../generated/api/ForkAlphaPreviewRequest";
import type { ForkAlphaPreviewView } from "../../../../generated/api/ForkAlphaPreviewView";
import type { PreparedCurriculumAssignmentView } from "../../../../generated/api/PreparedCurriculumAssignmentView";
import type { PreparedCurriculumCourseView } from "../../../../generated/api/PreparedCurriculumCourseView";
import type { ReconcileCurriculumAdoptionCommand } from "../../../../generated/api/ReconcileCurriculumAdoptionCommand";
import type { SourceDerivedAssignmentPreviewRequest } from "../../../../generated/api/SourceDerivedAssignmentPreviewRequest";
import type { SourceDerivedAssignmentPreviewView } from "../../../../generated/api/SourceDerivedAssignmentPreviewView";
import type { ObservedAlphaSource } from "../../../../generated/api/ObservedAlphaSource";
import type { ObservedBlueprintSource } from "../../../../generated/api/ObservedBlueprintSource";
import type { ObservedAssignmentRevision } from "../../../../generated/api/ObservedAssignmentRevision";
import type { ResolvedRelativeAssignmentSchedule } from "../../../../generated/api/ResolvedRelativeAssignmentSchedule";
import type { ResolvedRelativeScheduleMoment } from "../../../../generated/api/ResolvedRelativeScheduleMoment";
import {
  DecodeError,
  decodeBoolean,
  decodeNullable,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
  decodeRecord,
} from "../../decoder";
import { decodeAssignmentReference } from "../catalog_course";
import { decodeAssignmentTeachingSettingsValidationFailure } from "../assignment_teaching_delivery";
import { decodeCourseTerm } from "../course_term";
import { decodeBoundedArray, field, requireOnlyFields } from "../shared";

const MAX_INDEX = MAX_ASSIGNMENT_ORDERED_ENTRIES - 1;
const MAX_CANDIDATE_INDEX = MAX_ASSIGNMENT_CANDIDATES_PER_SELECTION_GROUP - 1;
const QUESTION_ID = /^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u;
const LOCAL_DATE_TIME = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}$/u;
const REVISION = /^[1-9][0-9]{0,18}$/u;

function closed(
  value: unknown,
  path: string,
  keys: ReadonlyArray<string>,
): Record<string, unknown> {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, keys);
  for (const key of keys) field(record, key, path);
  return record;
}

function routeReference(value: unknown, path: string, prefix: "C" | "A" | "AC" | "BP"): string {
  const reference = decodeString(value, path);
  const match = new RegExp(`^${prefix}-([1-9][0-9]{0,9})$`, "u").exec(reference);
  if (match === null || Number(match[1]) > 2_147_483_647) {
    throw new DecodeError(path, `a canonical ${prefix} route reference`);
  }
  return reference;
}

function revision(value: unknown, path: string): string {
  const valueString = decodeString(value, path);
  if (!REVISION.test(valueString) || BigInt(valueString) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a canonical positive PostgreSQL bigint revision");
  }
  return valueString;
}

function questionId(value: unknown, path: string): string {
  const id = decodeString(value, path);
  if (!QUESTION_ID.test(id)) throw new DecodeError(path, "a canonical public Question ID");
  return id;
}

function idempotencyKey(value: unknown, path: string): string {
  const key = decodeString(value, path);
  if (
    !/^[A-Za-z0-9._-]+$/u.test(key) ||
    new TextEncoder().encode(key).length > MAX_CURRICULUM_ADOPTION_IDEMPOTENCY_KEY_BYTES
  ) {
    throw new DecodeError(path, "a nonempty bounded idempotency key");
  }
  return key;
}

export function decodeCurriculumAdoptionIdempotencyKey(
  value: unknown,
  path = "request.idempotencyKey",
): string {
  return idempotencyKey(value, path);
}

export function decodeCurriculumCourseReference(value: unknown, path = "course"): string {
  return routeReference(value, path, "C");
}

function index(value: unknown, path: string, maximum: number): number {
  const decoded = decodeSafeInteger(value, path);
  if (decoded < 0 || decoded > maximum) throw new DecodeError(path, "a bounded nonnegative index");
  return decoded;
}

function source(value: unknown, path: string): AssignmentDefinitionSourceView {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "blueprint") {
    requireOnlyFields(record, path, ["kind", "reference", "revision"]);
    return {
      kind,
      reference: routeReference(field(record, "reference", path), `${path}.reference`, "BP"),
      revision: revision(field(record, "revision", path), `${path}.revision`),
    };
  }
  if (kind === "alpha") {
    requireOnlyFields(record, path, [
      "kind",
      "reference",
      "revision",
      "moduleIndex",
      "assignmentIndex",
    ]);
    return {
      kind,
      reference: routeReference(field(record, "reference", path), `${path}.reference`, "AC"),
      revision: revision(field(record, "revision", path), `${path}.revision`),
      moduleIndex: index(field(record, "moduleIndex", path), `${path}.moduleIndex`, MAX_INDEX),
      assignmentIndex: index(
        field(record, "assignmentIndex", path),
        `${path}.assignmentIndex`,
        MAX_INDEX,
      ),
    };
  }
  throw new DecodeError(`${path}.kind`, "a known assignment source kind");
}

function observedAlpha(value: unknown, path: string): ObservedAlphaSource {
  const record = closed(value, path, ["reference", "revision"]);
  return {
    reference: routeReference(field(record, "reference", path), `${path}.reference`, "AC"),
    revision: revision(field(record, "revision", path), `${path}.revision`),
  };
}

function observedBlueprint(value: unknown, path: string): ObservedBlueprintSource {
  const record = closed(value, path, ["reference", "revision"]);
  return {
    reference: routeReference(field(record, "reference", path), `${path}.reference`, "BP"),
    revision: revision(field(record, "revision", path), `${path}.revision`),
  };
}

function observedAssignment(value: unknown, path: string): ObservedAssignmentRevision {
  const record = closed(value, path, ["assignment", "revision"]);
  return {
    assignment: decodeAssignmentReference(field(record, "assignment", path), `${path}.assignment`),
    revision: revision(field(record, "revision", path), `${path}.revision`),
  };
}

function pinPosition(value: unknown, path: string): CurriculumPinPosition {
  const record = closed(value, path, [
    "moduleIndex",
    "assignmentIndex",
    "entryIndex",
    "candidateIndex",
  ]);
  return {
    moduleIndex: decodeNullable(
      field(record, "moduleIndex", path),
      `${path}.moduleIndex`,
      (item, itemPath) => index(item, itemPath, MAX_INDEX),
    ),
    assignmentIndex: index(
      field(record, "assignmentIndex", path),
      `${path}.assignmentIndex`,
      MAX_INDEX,
    ),
    entryIndex: index(field(record, "entryIndex", path), `${path}.entryIndex`, MAX_INDEX),
    candidateIndex: decodeNullable(
      field(record, "candidateIndex", path),
      `${path}.candidateIndex`,
      (item, itemPath) => index(item, itemPath, MAX_CANDIDATE_INDEX),
    ),
  };
}

function replacements(value: unknown, path: string): Array<CurriculumPinReplacement> {
  const decoded = decodeBoundedArray(
    value,
    path,
    MAX_ASSIGNMENT_TOTAL_SELECTION_CANDIDATES,
    (entry, entryPath) => {
      const record = closed(entry, entryPath, ["position", "question"]);
      return {
        position: pinPosition(field(record, "position", entryPath), `${entryPath}.position`),
        question: questionId(field(record, "question", entryPath), `${entryPath}.question`),
      };
    },
  );
  const positions = decoded.map((entry) => JSON.stringify(entry.position));
  if (new Set(positions).size !== positions.length)
    throw new DecodeError(path, "unique curriculum pin positions");
  return decoded;
}

function pinRecovery(value: unknown, path: string): UnavailablePinRecoveryAction {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "selectReplacementQuestion") {
    requireOnlyFields(record, path, ["kind", "position", "candidates"]);
    const candidates = decodeBoundedArray(
      field(record, "candidates", path),
      `${path}.candidates`,
      1_024,
      questionId,
    );
    if (candidates.length === 0 || new Set(candidates).size !== candidates.length)
      throw new DecodeError(`${path}.candidates`, "nonempty unique replacement questions");
    return {
      kind: "selectReplacementQuestion",
      position: pinPosition(field(record, "position", path), `${path}.position`),
      candidates,
    };
  }
  throw new DecodeError(`${path}.kind`, "selectReplacementQuestion");
}

function preservedRecovery(value: unknown, path: string): PreservedAssignmentRecoveryAction {
  const record = closed(value, path, ["kind"]);
  if (record.kind !== "createSourceDerivedAssignment")
    throw new DecodeError(`${path}.kind`, "createSourceDerivedAssignment");
  return { kind: "createSourceDerivedAssignment" };
}

function scheduleMoment(value: unknown, path: string): ResolvedRelativeScheduleMoment {
  const record = closed(value, path, ["local", "timestamp"]);
  const local = decodeString(field(record, "local", path), `${path}.local`);
  if (!LOCAL_DATE_TIME.test(local))
    throw new DecodeError(`${path}.local`, "a canonical local date-time");
  return {
    local,
    timestamp: decodeSafeInteger(field(record, "timestamp", path), `${path}.timestamp`),
  };
}

function schedule(value: unknown, path: string): ResolvedRelativeAssignmentSchedule {
  const record = closed(value, path, ["timeZone", "availableAt", "dueAt", "closesAt"]);
  const timeZone = decodeString(field(record, "timeZone", path), `${path}.timeZone`);
  if (timeZone.length === 0 || timeZone.length > 255 || timeZone.trim() !== timeZone)
    throw new DecodeError(`${path}.timeZone`, "a trimmed IANA time-zone name");
  return {
    timeZone,
    availableAt: decodeNullable(
      field(record, "availableAt", path),
      `${path}.availableAt`,
      scheduleMoment,
    ),
    dueAt: decodeNullable(field(record, "dueAt", path), `${path}.dueAt`, scheduleMoment),
    closesAt: decodeNullable(field(record, "closesAt", path), `${path}.closesAt`, scheduleMoment),
  };
}

function correction(value: unknown, path: string): CurriculumScheduleCorrection {
  const record = closed(value, path, ["correction"]);
  const decoded = decodeAssignmentTeachingSettingsValidationFailure(
    field(record, "correction", path),
    `${path}.correction`,
  );
  if (decoded.message.length > 160)
    throw new DecodeError(`${path}.correction.message`, "at most 160 characters");
  return { correction: decoded };
}

function witness(value: unknown, path: string): CourseScheduleWitness {
  const record = closed(value, path, ["course", "scheduleRevision", "assignmentRevisions"]);
  const assignmentRevisions = decodeBoundedArray(
    field(record, "assignmentRevisions", path),
    `${path}.assignmentRevisions`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    observedAssignment,
  );
  const assignments = assignmentRevisions.map((item) => item.assignment);
  if (new Set(assignments).size !== assignments.length)
    throw new DecodeError(`${path}.assignmentRevisions`, "unique assignment revisions");
  return {
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    scheduleRevision: revision(field(record, "scheduleRevision", path), `${path}.scheduleRevision`),
    assignmentRevisions,
  };
}

function preparedAssignment(value: unknown, path: string): PreparedCurriculumAssignmentView {
  const record = closed(value, path, ["title", "schedule"]);
  return {
    title: boundedText(field(record, "title", path), `${path}.title`),
    schedule: schedule(field(record, "schedule", path), `${path}.schedule`),
  };
}

function preparedCourse(value: unknown, path: string): PreparedCurriculumCourseView {
  const record = closed(value, path, ["title", "assignments"]);
  return {
    title: boundedText(field(record, "title", path), `${path}.title`),
    assignments: decodeBoundedArray(
      field(record, "assignments", path),
      `${path}.assignments`,
      MAX_ASSIGNMENT_ORDERED_ENTRIES,
      preparedAssignment,
    ),
  };
}

function boundedText(value: unknown, path: string): string {
  const text = decodeString(value, path);
  if (text.length === 0 || text.trim() !== text || Array.from(text).length > 200)
    throw new DecodeError(path, "trimmed nonempty text within 200 Unicode scalars");
  return text;
}

function assignmentView(value: unknown, path: string): CurriculumAssignmentView {
  const record = closed(value, path, ["reference", "title", "revision", "schedule"]);
  return {
    reference: decodeAssignmentReference(field(record, "reference", path), `${path}.reference`),
    title: boundedText(field(record, "title", path), `${path}.title`),
    revision: revision(field(record, "revision", path), `${path}.revision`),
    schedule: schedule(field(record, "schedule", path), `${path}.schedule`),
  };
}

function importSource(value: unknown, path: string): CurriculumAssignmentImportSourceView {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "reusable") {
    requireOnlyFields(record, path, ["kind", "definition"]);
    return { kind, definition: source(field(record, "definition", path), `${path}.definition`) };
  }
  if (kind === "rollover") {
    requireOnlyFields(record, path, ["kind", "source"]);
    const sourceRecord = closed(field(record, "source", path), `${path}.source`, ["assignment"]);
    return {
      kind,
      source: {
        assignment: observedAssignment(
          field(sourceRecord, "assignment", `${path}.source`),
          `${path}.source.assignment`,
        ),
      },
    };
  }
  throw new DecodeError(`${path}.kind`, "a known curriculum import source kind");
}

function origin(value: unknown, path: string): CurriculumCourseImportOriginView {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "ordinary") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind };
  }
  if (kind === "alpha") {
    requireOnlyFields(record, path, ["kind", "source"]);
    return { kind, source: observedAlpha(field(record, "source", path), `${path}.source`) };
  }
  if (kind === "rollover") {
    requireOnlyFields(record, path, ["kind", "source"]);
    const sourceRecord = closed(field(record, "source", path), `${path}.source`, [
      "sourceSchedule",
    ]);
    return {
      kind,
      source: {
        sourceSchedule: witness(
          field(sourceRecord, "sourceSchedule", `${path}.source`),
          `${path}.source.sourceSchedule`,
        ),
      },
    };
  }
  throw new DecodeError(`${path}.kind`, "a known curriculum course origin kind");
}

function importView(value: unknown, path: string): CurriculumImportView {
  const record = closed(value, path, [
    "assignment",
    "source",
    "revision",
    "reusableMeaningMatchesBaseline",
  ]);
  return {
    assignment: decodeAssignmentReference(field(record, "assignment", path), `${path}.assignment`),
    source: importSource(field(record, "source", path), `${path}.source`),
    revision: revision(field(record, "revision", path), `${path}.revision`),
    reusableMeaningMatchesBaseline: decodeBoolean(
      field(record, "reusableMeaningMatchesBaseline", path),
      `${path}.reusableMeaningMatchesBaseline`,
    ),
  };
}

function receipt(value: unknown, path: string): CurriculumAdoptionReceiptBinding {
  const record = closed(value, path, ["idempotencyKey"]);
  return {
    idempotencyKey: idempotencyKey(field(record, "idempotencyKey", path), `${path}.idempotencyKey`),
  };
}

function decision(value: unknown, path: string): AssignmentFastForwardDecision {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "eligible") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind };
  }
  if (kind === "divergent" || kind === "issuedWork") {
    requireOnlyFields(record, path, ["kind", "recovery"]);
    return {
      kind,
      recovery: preservedRecovery(field(record, "recovery", path), `${path}.recovery`),
    };
  }
  if (kind === "unavailablePin") {
    requireOnlyFields(record, path, ["kind", "recovery"]);
    return { kind, recovery: pinRecovery(field(record, "recovery", path), `${path}.recovery`) };
  }
  if (kind === "sourceRevisionDrift") {
    requireOnlyFields(record, path, ["kind", "source"]);
    return { kind, source: source(field(record, "source", path), `${path}.source`) };
  }
  throw new DecodeError(`${path}.kind`, "a known fast-forward decision");
}

function previewCommon(
  value: unknown,
  path: string,
  keys: ReadonlyArray<string>,
): Record<string, unknown> {
  return closed(value, path, keys);
}

export function decodeForkAlphaPreviewRequest(
  value: unknown,
  path = "request",
): ForkAlphaPreviewRequest {
  const record = closed(value, path, ["source", "replacements"]);
  return {
    source: observedAlpha(field(record, "source", path), `${path}.source`),
    replacements: replacements(field(record, "replacements", path), `${path}.replacements`),
  };
}

export function decodeBlueprintInstantiationPreviewRequest(
  value: unknown,
  path = "request",
): BlueprintInstantiationPreviewRequest {
  const record = closed(value, path, ["source", "course", "targetTerm", "replacements"]);
  return {
    source: observedBlueprint(field(record, "source", path), `${path}.source`),
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    targetTerm: decodeCourseTerm(field(record, "targetTerm", path), `${path}.targetTerm`),
    replacements: replacements(field(record, "replacements", path), `${path}.replacements`),
  };
}

export function decodeAlphaInstantiationPreviewRequest(
  value: unknown,
  path = "request",
): AlphaInstantiationPreviewRequest {
  const record = closed(value, path, ["source", "title", "targetTerm", "replacements"]);
  return {
    source: observedAlpha(field(record, "source", path), `${path}.source`),
    title: boundedText(field(record, "title", path), `${path}.title`),
    targetTerm: decodeCourseTerm(field(record, "targetTerm", path), `${path}.targetTerm`),
    replacements: replacements(field(record, "replacements", path), `${path}.replacements`),
  };
}

export function decodeCourseRolloverPreviewRequest(
  value: unknown,
  path = "request",
): CourseRolloverPreviewRequest {
  const record = closed(value, path, ["witness", "title", "targetTerm", "replacements"]);
  return {
    witness: witness(field(record, "witness", path), `${path}.witness`),
    title: boundedText(field(record, "title", path), `${path}.title`),
    targetTerm: decodeCourseTerm(field(record, "targetTerm", path), `${path}.targetTerm`),
    replacements: replacements(field(record, "replacements", path), `${path}.replacements`),
  };
}

export function decodeCourseTermShiftPreviewRequest(
  value: unknown,
  path = "request",
): CourseTermShiftPreviewRequest {
  const record = closed(value, path, ["witness", "targetTerm"]);
  return {
    witness: witness(field(record, "witness", path), `${path}.witness`),
    targetTerm: decodeCourseTerm(field(record, "targetTerm", path), `${path}.targetTerm`),
  };
}

export function decodeAssignmentFastForwardPreviewRequest(
  value: unknown,
  path = "request",
): import("../../../../generated/api/AssignmentFastForwardPreviewRequest").AssignmentFastForwardPreviewRequest {
  const record = closed(value, path, ["course", "assignment", "importRevision", "source"]);
  return {
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    assignment: observedAssignment(field(record, "assignment", path), `${path}.assignment`),
    importRevision: revision(field(record, "importRevision", path), `${path}.importRevision`),
    source: source(field(record, "source", path), `${path}.source`),
  };
}

export function decodeSourceDerivedAssignmentPreviewRequest(
  value: unknown,
  path = "request",
): SourceDerivedAssignmentPreviewRequest {
  const record = closed(value, path, ["course", "source", "replacements"]);
  return {
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    source: source(field(record, "source", path), `${path}.source`),
    replacements: replacements(field(record, "replacements", path), `${path}.replacements`),
  };
}

export function decodeReconcileCurriculumAdoptionCommand(
  value: unknown,
  path = "request",
): ReconcileCurriculumAdoptionCommand {
  const record = closed(value, path, ["receipt"]);
  return { receipt: receipt(field(record, "receipt", path), `${path}.receipt`) };
}

export function decodeForkAlphaPreviewView(
  value: unknown,
  path = "response",
): ForkAlphaPreviewView {
  const record = previewCommon(value, path, [
    "source",
    "resultingAlphaTitle",
    "replacements",
    "pinCorrection",
  ]);
  return {
    source: observedAlpha(field(record, "source", path), `${path}.source`),
    resultingAlphaTitle: boundedText(
      field(record, "resultingAlphaTitle", path),
      `${path}.resultingAlphaTitle`,
    ),
    replacements: replacements(field(record, "replacements", path), `${path}.replacements`),
    pinCorrection: decodeNullable(
      field(record, "pinCorrection", path),
      `${path}.pinCorrection`,
      pinRecovery,
    ),
  };
}

export function decodeBlueprintInstantiationPreviewView(
  value: unknown,
  path = "response",
): BlueprintInstantiationPreviewView {
  const record = previewCommon(value, path, [
    "source",
    "course",
    "targetTerm",
    "witness",
    "assignment",
    "replacements",
    "corrections",
    "pinCorrection",
  ]);
  return {
    source: observedBlueprint(field(record, "source", path), `${path}.source`),
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    targetTerm: decodeCourseTerm(field(record, "targetTerm", path), `${path}.targetTerm`),
    witness: witness(field(record, "witness", path), `${path}.witness`),
    assignment: preparedAssignment(field(record, "assignment", path), `${path}.assignment`),
    replacements: replacements(field(record, "replacements", path), `${path}.replacements`),
    corrections: decodeBoundedArray(
      field(record, "corrections", path),
      `${path}.corrections`,
      MAX_ASSIGNMENT_ORDERED_ENTRIES,
      correction,
    ),
    pinCorrection: decodeNullable(
      field(record, "pinCorrection", path),
      `${path}.pinCorrection`,
      pinRecovery,
    ),
  };
}

export function decodeAlphaInstantiationPreviewView(
  value: unknown,
  path = "response",
): AlphaInstantiationPreviewView {
  const record = previewCommon(value, path, [
    "source",
    "targetTerm",
    "course",
    "replacements",
    "corrections",
    "pinCorrection",
  ]);
  return {
    source: observedAlpha(field(record, "source", path), `${path}.source`),
    targetTerm: decodeCourseTerm(field(record, "targetTerm", path), `${path}.targetTerm`),
    course: preparedCourse(field(record, "course", path), `${path}.course`),
    replacements: replacements(field(record, "replacements", path), `${path}.replacements`),
    corrections: decodeBoundedArray(
      field(record, "corrections", path),
      `${path}.corrections`,
      MAX_ASSIGNMENT_ORDERED_ENTRIES,
      correction,
    ),
    pinCorrection: decodeNullable(
      field(record, "pinCorrection", path),
      `${path}.pinCorrection`,
      pinRecovery,
    ),
  };
}

export function decodeCourseRolloverPreviewView(
  value: unknown,
  path = "response",
): CourseRolloverPreviewView {
  const record = previewCommon(value, path, [
    "witness",
    "targetTerm",
    "course",
    "replacements",
    "corrections",
    "pinCorrection",
  ]);
  return {
    witness: witness(field(record, "witness", path), `${path}.witness`),
    targetTerm: decodeCourseTerm(field(record, "targetTerm", path), `${path}.targetTerm`),
    course: preparedCourse(field(record, "course", path), `${path}.course`),
    replacements: replacements(field(record, "replacements", path), `${path}.replacements`),
    corrections: decodeBoundedArray(
      field(record, "corrections", path),
      `${path}.corrections`,
      MAX_ASSIGNMENT_ORDERED_ENTRIES,
      correction,
    ),
    pinCorrection: decodeNullable(
      field(record, "pinCorrection", path),
      `${path}.pinCorrection`,
      pinRecovery,
    ),
  };
}

export function decodeCourseTermShiftPreviewView(
  value: unknown,
  path = "response",
): CourseTermShiftPreviewView {
  const record = previewCommon(value, path, [
    "witness",
    "targetTerm",
    "assignments",
    "corrections",
  ]);
  return {
    witness: witness(field(record, "witness", path), `${path}.witness`),
    targetTerm: decodeCourseTerm(field(record, "targetTerm", path), `${path}.targetTerm`),
    assignments: decodeBoundedArray(
      field(record, "assignments", path),
      `${path}.assignments`,
      MAX_ASSIGNMENT_ORDERED_ENTRIES,
      assignmentView,
    ),
    corrections: decodeBoundedArray(
      field(record, "corrections", path),
      `${path}.corrections`,
      MAX_ASSIGNMENT_ORDERED_ENTRIES,
      correction,
    ),
  };
}

export function decodeCourseTermShiftPreviewOutcome(
  value: unknown,
  path = "response",
): CourseTermShiftPreviewOutcome {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "eligible") {
    requireOnlyFields(record, path, ["kind", "preview"]);
    return {
      kind,
      preview: decodeCourseTermShiftPreviewView(field(record, "preview", path), `${path}.preview`),
    };
  }
  if (kind === "ineligible") {
    requireOnlyFields(record, path, ["kind", "course", "reason", "recovery"]);
    return {
      kind,
      course: routeReference(field(record, "course", path), `${path}.course`, "C"),
      reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, [
        "issuedWork",
      ] as const),
      recovery: decodeStringEnum(field(record, "recovery", path), `${path}.recovery`, [
        "rolloverCourse",
      ] as const),
    };
  }
  throw new DecodeError(`${path}.kind`, "eligible or ineligible");
}

export function decodeAssignmentFastForwardPreviewView(
  value: unknown,
  path = "response",
): AssignmentFastForwardPreviewView {
  const record = previewCommon(value, path, [
    "course",
    "assignment",
    "importRevision",
    "source",
    "witness",
    "decision",
  ]);
  return {
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    assignment: observedAssignment(field(record, "assignment", path), `${path}.assignment`),
    importRevision: revision(field(record, "importRevision", path), `${path}.importRevision`),
    source: source(field(record, "source", path), `${path}.source`),
    witness: witness(field(record, "witness", path), `${path}.witness`),
    decision: decision(field(record, "decision", path), `${path}.decision`),
  };
}

export function decodeSourceDerivedAssignmentPreviewView(
  value: unknown,
  path = "response",
): SourceDerivedAssignmentPreviewView {
  const record = previewCommon(value, path, [
    "course",
    "source",
    "witness",
    "assignment",
    "replacements",
    "corrections",
    "pinCorrection",
  ]);
  return {
    course: routeReference(field(record, "course", path), `${path}.course`, "C"),
    source: source(field(record, "source", path), `${path}.source`),
    witness: witness(field(record, "witness", path), `${path}.witness`),
    assignment: preparedAssignment(field(record, "assignment", path), `${path}.assignment`),
    replacements: replacements(field(record, "replacements", path), `${path}.replacements`),
    corrections: decodeBoundedArray(
      field(record, "corrections", path),
      `${path}.corrections`,
      MAX_ASSIGNMENT_ORDERED_ENTRIES,
      correction,
    ),
    pinCorrection: decodeNullable(
      field(record, "pinCorrection", path),
      `${path}.pinCorrection`,
      pinRecovery,
    ),
  };
}

export function decodeCurriculumCourseImportView(
  value: unknown,
  path = "response",
): CurriculumCourseImportView {
  const record = closed(value, path, ["witness", "origin", "term", "assignments"]);
  const decodedWitness = witness(field(record, "witness", path), `${path}.witness`);
  const assignments = decodeBoundedArray(
    field(record, "assignments", path),
    `${path}.assignments`,
    MAX_ASSIGNMENT_ORDERED_ENTRIES,
    importView,
  );
  if (
    assignments.length === 0 ||
    new Set(assignments.map((item) => item.assignment)).size !== assignments.length
  )
    throw new DecodeError(`${path}.assignments`, "nonempty unique imports");
  const witnessedAssignments = new Set(
    decodedWitness.assignmentRevisions.map((item) => item.assignment),
  );
  if (assignments.some((item) => !witnessedAssignments.has(item.assignment))) {
    throw new DecodeError(
      `${path}.assignments`,
      "imports whose assignments are present in the course schedule witness",
    );
  }
  return {
    witness: decodedWitness,
    origin: origin(field(record, "origin", path), `${path}.origin`),
    term: decodeCourseTerm(field(record, "term", path), `${path}.term`),
    assignments,
  };
}

export function decodeCurriculumAdoptionReconciliationResult(
  value: unknown,
  path = "response",
): CurriculumAdoptionReconciliationResult {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "alreadyConsistent") {
    requireOnlyFields(record, path, ["kind", "receipt"]);
    return { kind, receipt: receipt(field(record, "receipt", path), `${path}.receipt`) };
  }
  if (kind === "repaired") {
    requireOnlyFields(record, path, ["kind", "receipt", "projections"]);
    const projections = decodeBoundedArray(
      field(record, "projections", path),
      `${path}.projections`,
      MAX_ASSIGNMENT_ORDERED_ENTRIES,
      (item, itemPath): CurriculumAdoptionRepairedProjection => {
        const projection = closed(item, itemPath, ["kind", "assignment"]);
        return {
          kind: decodeStringEnum(field(projection, "kind", itemPath), `${itemPath}.kind`, [
            "assignmentImportCurrent",
          ] as const),
          assignment: decodeAssignmentReference(
            field(projection, "assignment", itemPath),
            `${itemPath}.assignment`,
          ),
        };
      },
    );
    return {
      kind,
      receipt: receipt(field(record, "receipt", path), `${path}.receipt`),
      projections,
    };
  }
  throw new DecodeError(`${path}.kind`, "alreadyConsistent or repaired");
}
