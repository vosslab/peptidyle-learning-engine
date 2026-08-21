// Strict browser decoders for generated WP-PROF-T3 preview-plane DTOs.

import { MAX_PREVIEW_SUBJECT_GROUPS } from "../../../generated/api/MAX_PREVIEW_SUBJECT_GROUPS";
import { MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS } from "../../../generated/api/MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS";
import { MAX_TEACHING_PAGE_SIZE } from "../../../generated/api/MAX_TEACHING_PAGE_SIZE";
import type { DerivedPreviewSubjectRequest } from "../../../generated/api/DerivedPreviewSubjectRequest";
import type { InstructorPreviewSchedulePage } from "../../../generated/api/InstructorPreviewSchedulePage";
import type { InstructorPreviewScheduleRow } from "../../../generated/api/InstructorPreviewScheduleRow";
import type { PreviewAccommodationComparison } from "../../../generated/api/PreviewAccommodationComparison";
import type { PreviewDisclosureProjection } from "../../../generated/api/PreviewDisclosureProjection";
import type { PreviewEvaluation } from "../../../generated/api/PreviewEvaluation";
import type { PreviewPlaneResponse } from "../../../generated/api/PreviewPlaneResponse";
import type { PreviewScheduleProjection } from "../../../generated/api/PreviewScheduleProjection";
import type { PreviewSelectedMoment } from "../../../generated/api/PreviewSelectedMoment";
import type { PreviewSubject } from "../../../generated/api/PreviewSubject";
import type { SyntheticPreviewSubjectRequest } from "../../../generated/api/SyntheticPreviewSubjectRequest";
import {
  DecodeError,
  decodeBoolean,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeAssignmentPolicyPatchUpdateRequest } from "./teaching_operations";
import { decodeBoundedArray, decodeCursor, field, requireOnlyFields } from "./shared";

const MAX_ROUTE_REFERENCE = 2_147_483_647;
const POLICY_SOURCES = [
  "base",
  "groupSchedule",
  "groupAccommodation",
  "individualException",
] as const;
const GROUP_PURPOSES = ["section", "lab", "cohort", "accommodation", "work"] as const;
const DISCLOSURE_MOMENTS = ["now", "due", "close"] as const;

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

function reference(value: unknown, path: string, prefix: string): string {
  const parsed = decodeString(value, path);
  const pattern = new RegExp(`^${prefix}-[1-9][0-9]{0,9}$`, "u");
  if (!pattern.test(parsed)) throw new DecodeError(path, `a ${prefix}- prefixed route reference`);
  const number = Number(parsed.slice(prefix.length + 1));
  if (!Number.isSafeInteger(number) || number > MAX_ROUTE_REFERENCE) {
    throw new DecodeError(path, "a positive 31-bit route reference");
  }
  return parsed;
}

function revision(value: unknown, path: string): string {
  const parsed = decodeString(value, path);
  if (!/^[1-9][0-9]{0,18}$/u.test(parsed) || BigInt(parsed) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a canonical positive PostgreSQL bigint revision");
  }
  return parsed;
}

function label(value: unknown, path: string): string {
  const parsed = decodeString(value, path);
  if (parsed.trim() !== parsed || parsed.length === 0) {
    throw new DecodeError(path, "trimmed nonblank text");
  }
  if (Array.from(parsed).length > MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS) {
    throw new DecodeError(path, "a bounded display label");
  }
  return parsed;
}

function courseLocalDateTime(value: unknown, path: string): string {
  const parsed = decodeString(value, path);
  const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})\.(\d{3})$/u.exec(parsed);
  if (match === null) throw new DecodeError(path, "an exact local date-time");
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const hour = Number(match[4]);
  const minute = Number(match[5]);
  const second = Number(match[6]);
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const monthLengths = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  const monthLength = monthLengths[month - 1];
  if (
    year === 0 ||
    monthLength === undefined ||
    day < 1 ||
    day > monthLength ||
    hour > 23 ||
    minute > 59 ||
    second > 59
  ) {
    throw new DecodeError(path, "an exact local date-time");
  }
  return parsed;
}

function selectedMoment(value: unknown, path: string): PreviewSelectedMoment {
  const record = closed(value, path, ["value", "timeZone"]);
  const timeZone = label(record.timeZone, `${path}.timeZone`);
  if (Array.from(timeZone).length > 255)
    throw new DecodeError(`${path}.timeZone`, "a bounded IANA zone");
  return { value: courseLocalDateTime(record.value, `${path}.value`), timeZone };
}

function policySource(
  value: unknown,
  path: string,
): PreviewScheduleProjection["availableAt"]["source"] {
  return decodeStringEnum(value, path, POLICY_SOURCES);
}

function timeField(value: unknown, path: string): PreviewScheduleProjection["availableAt"] {
  const record = closed(value, path, ["value", "source"]);
  return {
    value: decodeNullable(record.value, `${path}.value`, courseLocalDateTime),
    source: policySource(record.source, `${path}.source`),
  };
}

function limitField(value: unknown, path: string): PreviewScheduleProjection["timeLimitSeconds"] {
  const record = closed(value, path, ["value", "source"]);
  const limit = decodeNullable(record.value, `${path}.value`, decodeSafeInteger);
  if (limit !== null && limit < 1)
    throw new DecodeError(`${path}.value`, "a positive safe integer");
  return { value: limit, source: policySource(record.source, `${path}.source`) };
}

function schedule(value: unknown, path: string): PreviewScheduleProjection {
  const record = closed(value, path, [
    "availableAt",
    "dueAt",
    "closesAt",
    "timeLimitSeconds",
    "attemptLimit",
    "lateSubmission",
    "deadlineBehavior",
  ]);
  const lateSubmission = closed(record.lateSubmission, `${path}.lateSubmission`, [
    "value",
    "source",
  ]);
  const deadlineBehavior = closed(record.deadlineBehavior, `${path}.deadlineBehavior`, [
    "value",
    "source",
  ]);
  return {
    availableAt: timeField(record.availableAt, `${path}.availableAt`),
    dueAt: timeField(record.dueAt, `${path}.dueAt`),
    closesAt: timeField(record.closesAt, `${path}.closesAt`),
    timeLimitSeconds: limitField(record.timeLimitSeconds, `${path}.timeLimitSeconds`),
    attemptLimit: limitField(record.attemptLimit, `${path}.attemptLimit`),
    lateSubmission: {
      value: decodeStringEnum(lateSubmission.value, `${path}.lateSubmission.value`, [
        "accept",
        "markLate",
        "reject",
      ] as const),
      source: policySource(lateSubmission.source, `${path}.lateSubmission.source`),
    },
    deadlineBehavior: {
      value: decodeStringEnum(deadlineBehavior.value, `${path}.deadlineBehavior.value`, [
        "autoSubmit",
      ] as const),
      source: policySource(deadlineBehavior.source, `${path}.deadlineBehavior.source`),
    },
  };
}

function subject(value: unknown, path: string): PreviewSubject {
  const record = closed(value, path, [
    "kind",
    "assignment",
    "revision",
    "selectedMoment",
    "groups",
    "policy",
    "priorRunCount",
  ]);
  const groups = decodeBoundedArray(
    record.groups,
    `${path}.groups`,
    MAX_PREVIEW_SUBJECT_GROUPS,
    (entry, entryPath) => {
      const fact = closed(entry, entryPath, ["purpose"]);
      return { purpose: decodeStringEnum(fact.purpose, `${entryPath}.purpose`, GROUP_PURPOSES) };
    },
  );
  if (new Set(groups.map((group) => group.purpose)).size !== groups.length) {
    throw new DecodeError(`${path}.groups`, "unique identity-free group purposes");
  }
  return {
    kind: decodeStringEnum(record.kind, `${path}.kind`, ["synthetic", "derived"] as const),
    assignment: reference(record.assignment, `${path}.assignment`, "A"),
    revision: revision(record.revision, `${path}.revision`),
    selectedMoment: selectedMoment(record.selectedMoment, `${path}.selectedMoment`),
    groups,
    policy: schedule(record.policy, `${path}.policy`),
    priorRunCount: nonnegativeInteger(record.priorRunCount, `${path}.priorRunCount`),
  };
}

function nonnegativeInteger(value: unknown, path: string): number {
  const parsed = decodeSafeInteger(value, path);
  if (parsed < 0) throw new DecodeError(path, "a nonnegative safe integer");
  return parsed;
}

function disclosure(value: unknown, path: string): PreviewDisclosureProjection {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "available") {
    requireOnlyFields(record, path, ["kind", "moment", "flags"]);
    const flags = closed(field(record, "flags", path), `${path}.flags`, [
      "scoreShown",
      "correctnessShown",
      "feedbackShown",
      "solutionShown",
      "statisticsShown",
    ]);
    return {
      kind,
      moment: decodeStringEnum(field(record, "moment", path), `${path}.moment`, DISCLOSURE_MOMENTS),
      flags: {
        scoreShown: decodeBoolean(flags.scoreShown, `${path}.flags.scoreShown`),
        correctnessShown: decodeBoolean(flags.correctnessShown, `${path}.flags.correctnessShown`),
        feedbackShown: decodeBoolean(flags.feedbackShown, `${path}.flags.feedbackShown`),
        solutionShown: decodeBoolean(flags.solutionShown, `${path}.flags.solutionShown`),
        statisticsShown: decodeBoolean(flags.statisticsShown, `${path}.flags.statisticsShown`),
      },
    };
  }
  if (kind === "unavailable") {
    requireOnlyFields(record, path, ["kind", "moment", "reason"]);
    return {
      kind,
      moment: decodeStringEnum(field(record, "moment", path), `${path}.moment`, DISCLOSURE_MOMENTS),
      reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, [
        "boundaryMissing",
      ] as const),
    };
  }
  throw new DecodeError(`${path}.kind`, "a known disclosure projection kind");
}

function disclosures(value: unknown, path: string): Array<PreviewDisclosureProjection> {
  const decoded = decodeBoundedArray(value, path, DISCLOSURE_MOMENTS.length, disclosure);
  const moments = decoded.map((projection) => projection.moment);
  if (
    moments.length !== DISCLOSURE_MOMENTS.length ||
    moments.some((moment, index) => moment !== DISCLOSURE_MOMENTS[index])
  ) {
    throw new DecodeError(path, "one ordered projection for now, due, and close");
  }
  return decoded;
}

function evaluation(value: unknown, path: string): PreviewEvaluation {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "allowed") {
    requireOnlyFields(record, path, ["kind", "subject", "entitlement", "schedule", "disclosure"]);
    return {
      kind,
      subject: subject(field(record, "subject", path), `${path}.subject`),
      entitlement: decodeStringEnum(field(record, "entitlement", path), `${path}.entitlement`, [
        "courseWide",
        "groupAudience",
      ] as const),
      schedule: schedule(field(record, "schedule", path), `${path}.schedule`),
      disclosure: disclosures(field(record, "disclosure", path), `${path}.disclosure`),
    };
  }
  if (kind === "denied") {
    requireOnlyFields(record, path, ["kind", "reason"]);
    return {
      kind,
      reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, [
        "notEntitled",
        "staleRevision",
      ] as const),
    };
  }
  throw new DecodeError(`${path}.kind`, "allowed or denied");
}

function accommodation(value: unknown, path: string): PreviewAccommodationComparison {
  const record = closed(value, path, ["before", "after"]);
  return {
    before: schedule(record.before, `${path}.before`),
    after: schedule(record.after, `${path}.after`),
  };
}

export function decodePreviewPlaneResponse(
  value: unknown,
  path = "response",
): PreviewPlaneResponse {
  const record = closed(value, path, ["evaluation", "accommodation"]);
  const decodedEvaluation = evaluation(record.evaluation, `${path}.evaluation`);
  const decodedAccommodation = decodeNullable(
    record.accommodation,
    `${path}.accommodation`,
    accommodation,
  );
  if (decodedEvaluation.kind === "denied" && decodedAccommodation !== null) {
    throw new DecodeError(`${path}.accommodation`, "null when preview evaluation is denied");
  }
  return { evaluation: decodedEvaluation, accommodation: decodedAccommodation };
}

function scheduleRow(value: unknown, path: string): InstructorPreviewScheduleRow {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "granted") {
    requireOnlyFields(record, path, ["kind", "membership", "display", "entitlement", "schedule"]);
    return {
      kind,
      membership: reference(field(record, "membership", path), `${path}.membership`, "M"),
      display: label(field(record, "display", path), `${path}.display`),
      entitlement: decodeStringEnum(field(record, "entitlement", path), `${path}.entitlement`, [
        "courseWide",
        "groupAudience",
      ] as const),
      schedule: schedule(field(record, "schedule", path), `${path}.schedule`),
    };
  }
  if (kind === "denied") {
    requireOnlyFields(record, path, ["kind", "membership", "display", "reason"]);
    return {
      kind,
      membership: reference(field(record, "membership", path), `${path}.membership`, "M"),
      display: label(field(record, "display", path), `${path}.display`),
      reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, [
        "notEntitled",
      ] as const),
    };
  }
  throw new DecodeError(`${path}.kind`, "granted or denied");
}

export function decodeInstructorPreviewSchedulePage(
  value: unknown,
  path = "response",
): InstructorPreviewSchedulePage {
  const record = closed(value, path, ["revision", "rows", "nextCursor"]);
  return {
    revision: revision(record.revision, `${path}.revision`),
    rows: decodeBoundedArray(record.rows, `${path}.rows`, MAX_TEACHING_PAGE_SIZE, scheduleRow),
    nextCursor: decodeNullable(record.nextCursor, `${path}.nextCursor`, decodeCursor),
  };
}

export function decodeSyntheticPreviewSubjectRequest(
  value: unknown,
  path = "request",
): SyntheticPreviewSubjectRequest {
  const record = closed(value, path, [
    "assignment",
    "revision",
    "selectedMoment",
    "groups",
    "modifiers",
  ]);
  const groups = decodeBoundedArray(
    record.groups,
    `${path}.groups`,
    MAX_PREVIEW_SUBJECT_GROUPS,
    (entry, entryPath) => reference(entry, entryPath, "G"),
  );
  if (new Set(groups).size !== groups.length)
    throw new DecodeError(`${path}.groups`, "unique group references");
  return {
    assignment: reference(record.assignment, `${path}.assignment`, "A"),
    revision: revision(record.revision, `${path}.revision`),
    selectedMoment: selectedMoment(record.selectedMoment, `${path}.selectedMoment`),
    groups,
    modifiers: decodeAssignmentPolicyPatchUpdateRequest(record.modifiers, `${path}.modifiers`),
  };
}

export function decodeDerivedPreviewSubjectRequest(
  value: unknown,
  path = "request",
): DerivedPreviewSubjectRequest {
  const record = closed(value, path, ["assignment", "revision", "selectedMoment", "membership"]);
  return {
    assignment: reference(record.assignment, `${path}.assignment`, "A"),
    revision: revision(record.revision, `${path}.revision`),
    selectedMoment: selectedMoment(record.selectedMoment, `${path}.selectedMoment`),
    membership: reference(record.membership, `${path}.membership`, "M"),
  };
}
