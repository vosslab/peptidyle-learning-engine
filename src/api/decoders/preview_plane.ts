// Strict browser decoders for generated WP-INST-T3 preview-plane DTOs.

import { MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS } from "../../../generated/api/MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS";
import { MAX_TEACHING_PAGE_SIZE } from "../../../generated/api/MAX_TEACHING_PAGE_SIZE";
import type { DerivedPreviewSubjectRequest } from "../../../generated/api/DerivedPreviewSubjectRequest";
import type { InstructorPreviewSchedulePage } from "../../../generated/api/InstructorPreviewSchedulePage";
import type { InstructorPreviewScheduleRow } from "../../../generated/api/InstructorPreviewScheduleRow";
import type { PreviewAccommodationComparison } from "../../../generated/api/PreviewAccommodationComparison";
import type { StudentFeedbackReleaseView } from "../../../generated/api/StudentFeedbackReleaseView";
import type { PreviewEvaluation } from "../../../generated/api/PreviewEvaluation";
import type { PreviewPlaneResponse } from "../../../generated/api/PreviewPlaneResponse";
import type { EffectiveAssignmentPolicyView } from "../../../generated/api/EffectiveAssignmentPolicyView";
import type { PreviewSelectedMoment } from "../../../generated/api/PreviewSelectedMoment";
import type { StudentViewScenario } from "../../../generated/api/StudentViewScenario";
import type { StudentViewScenarioRequest } from "../../../generated/api/StudentViewScenarioRequest";
import type {
  QuestionPoolPreview,
  QuestionPoolPreviewQuestion,
  QuestionPoolPreviewRequest,
} from "../contracts";
import {
  DecodeError,
  decodeBoolean,
  decodeNullable,
  decodeRecord,
  decodeSafeInteger,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeSyntheticPreviewAccommodationAdjustmentRequest } from "./teaching_operations";
import {
  decodeBoundedArray,
  decodeCursor,
  decodeIdentifier,
  field,
  requireOnlyFields,
} from "./shared";

const MAX_ROUTE_REFERENCE = 2_147_483_647;
const POLICY_SOURCES = ["base", "accommodation"] as const;
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

function previewQuestion(value: unknown, path: string): QuestionPoolPreviewQuestion {
  const record = closed(value, path, ["questionId", "title"]);
  return {
    questionId: questionId(record.questionId, `${path}.questionId`),
    title: label(record.title, `${path}.title`),
  };
}

function questionId(value: unknown, path: string): string {
  const parsed = decodeString(value, path);
  if (!/^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u.test(parsed))
    throw new DecodeError(path, "a canonical Question ID");
  return parsed;
}

export function decodeQuestionPoolPreviewRequest(
  value: unknown,
  path = "request",
): QuestionPoolPreviewRequest {
  const record = closed(value, path, ["assignmentEntryId"]);
  return {
    assignmentEntryId: decodeIdentifier(record.assignmentEntryId, `${path}.assignmentEntryId`),
  };
}

export function decodeQuestionPoolPreview(value: unknown, path = "response"): QuestionPoolPreview {
  const record = closed(value, path, [
    "assignment",
    "revision",
    "assignmentEntryId",
    "questionPoolLabel",
    "selectionCount",
    "selectionRule",
    "entries",
    "selected",
  ]);
  const entries = decodeBoundedArray(
    record.entries,
    `${path}.entries`,
    MAX_TEACHING_PAGE_SIZE,
    previewQuestion,
  );
  const selected = decodeBoundedArray(
    record.selected,
    `${path}.selected`,
    MAX_TEACHING_PAGE_SIZE,
    previewQuestion,
  );
  const selectionCount = decodeSafeInteger(record.selectionCount, `${path}.selectionCount`);
  const assignmentEntryId = decodeIdentifier(record.assignmentEntryId, `${path}.assignmentEntryId`);
  if (selectionCount < 1 || selectionCount > entries.length || selected.length !== selectionCount)
    throw new DecodeError(
      `${path}.selectionCount`,
      "a valid selection count for the returned pool",
    );
  const selectedIds = new Set(selected.map((question) => question.questionId));
  if (
    selectedIds.size !== selected.length ||
    !selected.every((question) => entries.some((entry) => entry.questionId === question.questionId))
  )
    throw new DecodeError(`${path}.selected`, "unique Question Pool Item Question IDs");
  return {
    assignment: reference(record.assignment, `${path}.assignment`, "A"),
    revision: revision(record.revision, `${path}.revision`),
    assignmentEntryId,
    questionPoolLabel: label(record.questionPoolLabel, `${path}.questionPoolLabel`),
    selectionCount,
    selectionRule: selectionRule(record.selectionRule, `${path}.selectionRule`),
    entries,
    selected,
  };
}

function selectionRule(
  value: unknown,
  path: string,
): { readonly selectedQuestionOrder: "questionPoolOrder" | "randomOrder" } {
  const record = closed(value, path, ["selectedQuestionOrder"]);
  return {
    selectedQuestionOrder: decodeStringEnum(
      record.selectedQuestionOrder,
      `${path}.selectedQuestionOrder`,
      ["questionPoolOrder", "randomOrder"] as const,
    ),
  };
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

function courseLocalDateAndTime(value: unknown, path: string): string {
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
  return { value: courseLocalDateAndTime(record.value, `${path}.value`), timeZone };
}

function assignmentPolicySourceKind(
  value: unknown,
  path: string,
): EffectiveAssignmentPolicyView["availableAt"]["source"] {
  return decodeStringEnum(value, path, POLICY_SOURCES);
}

function timeField(value: unknown, path: string): EffectiveAssignmentPolicyView["availableAt"] {
  const record = closed(value, path, ["value", "source"]);
  return {
    value: decodeNullable(record.value, `${path}.value`, courseLocalDateAndTime),
    source: assignmentPolicySourceKind(record.source, `${path}.source`),
  };
}

function limitField(
  value: unknown,
  path: string,
): EffectiveAssignmentPolicyView["assignmentAttemptTimeLimitSeconds"] {
  const record = closed(value, path, ["value", "source"]);
  const limit = decodeNullable(record.value, `${path}.value`, decodeSafeInteger);
  if (limit !== null && limit < 1)
    throw new DecodeError(`${path}.value`, "a positive safe integer");
  return { value: limit, source: assignmentPolicySourceKind(record.source, `${path}.source`) };
}

function effective_assignment_policy(value: unknown, path: string): EffectiveAssignmentPolicyView {
  const record = closed(value, path, [
    "availableAt",
    "dueAt",
    "closesAt",
    "assignmentAttemptTimeLimitSeconds",
    "attemptLimit",
    "lateWorkRule",
    "assignmentDeadlineRule",
  ]);
  const lateWorkRule = closed(record.lateWorkRule, `${path}.lateWorkRule`, ["value", "source"]);
  const assignmentDeadlineRule = closed(
    record.assignmentDeadlineRule,
    `${path}.assignmentDeadlineRule`,
    ["value", "source"],
  );
  return {
    availableAt: timeField(record.availableAt, `${path}.availableAt`),
    dueAt: timeField(record.dueAt, `${path}.dueAt`),
    closesAt: timeField(record.closesAt, `${path}.closesAt`),
    assignmentAttemptTimeLimitSeconds: limitField(
      record.assignmentAttemptTimeLimitSeconds,
      `${path}.assignmentAttemptTimeLimitSeconds`,
    ),
    attemptLimit: limitField(record.attemptLimit, `${path}.attemptLimit`),
    lateWorkRule: {
      value: decodeStringEnum(lateWorkRule.value, `${path}.lateWorkRule.value`, [
        "accept",
        "markLate",
        "reject",
      ] as const),
      source: assignmentPolicySourceKind(lateWorkRule.source, `${path}.lateWorkRule.source`),
    },
    assignmentDeadlineRule: {
      value: decodeStringEnum(
        assignmentDeadlineRule.value,
        `${path}.assignmentDeadlineRule.value`,
        ["autoSubmit"] as const,
      ),
      source: assignmentPolicySourceKind(
        assignmentDeadlineRule.source,
        `${path}.assignmentDeadlineRule.source`,
      ),
    },
  };
}

function studentViewScenario(value: unknown, path: string): StudentViewScenario {
  const record = closed(value, path, [
    "kind",
    "assignment",
    "revision",
    "selectedMoment",
    "policy",
    "priorAssignmentAttemptCount",
  ]);
  return {
    kind: decodeStringEnum(record.kind, `${path}.kind`, ["synthetic", "derived"] as const),
    assignment: reference(record.assignment, `${path}.assignment`, "A"),
    revision: revision(record.revision, `${path}.revision`),
    selectedMoment: selectedMoment(record.selectedMoment, `${path}.selectedMoment`),
    policy: effective_assignment_policy(record.policy, `${path}.policy`),
    priorAssignmentAttemptCount: nonnegativeInteger(
      record.priorAssignmentAttemptCount,
      `${path}.priorAssignmentAttemptCount`,
    ),
  };
}

function nonnegativeInteger(value: unknown, path: string): number {
  const parsed = decodeSafeInteger(value, path);
  if (parsed < 0) throw new DecodeError(path, "a nonnegative safe integer");
  return parsed;
}

function student_feedback_release(value: unknown, path: string): StudentFeedbackReleaseView {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "available") {
    requireOnlyFields(record, path, ["kind", "moment", "flags"]);
    const flags = closed(field(record, "flags", path), `${path}.flags`, [
      "scoreShown",
      "correctnessShown",
      "feedbackShown",
      "questionAnswerShown",
      "questionAnswerExplanationShown",
      "statisticsShown",
    ]);
    return {
      kind,
      moment: decodeStringEnum(field(record, "moment", path), `${path}.moment`, DISCLOSURE_MOMENTS),
      flags: {
        scoreShown: decodeBoolean(flags.scoreShown, `${path}.flags.scoreShown`),
        correctnessShown: decodeBoolean(flags.correctnessShown, `${path}.flags.correctnessShown`),
        feedbackShown: decodeBoolean(flags.feedbackShown, `${path}.flags.feedbackShown`),
        questionAnswerShown: decodeBoolean(
          flags.questionAnswerShown,
          `${path}.flags.questionAnswerShown`,
        ),
        questionAnswerExplanationShown: decodeBoolean(
          flags.questionAnswerExplanationShown,
          `${path}.flags.questionAnswerExplanationShown`,
        ),
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
  throw new DecodeError(`${path}.kind`, "a known student_feedback_release projection kind");
}

function disclosures(value: unknown, path: string): Array<StudentFeedbackReleaseView> {
  const decoded = decodeBoundedArray(
    value,
    path,
    DISCLOSURE_MOMENTS.length,
    student_feedback_release,
  );
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
    requireOnlyFields(record, path, [
      "kind",
      "student_view_scenario",
      "active_student_course_membership",
      "effective_assignment_policy",
      "student_feedback_release",
    ]);
    return {
      kind,
      student_view_scenario: studentViewScenario(
        field(record, "student_view_scenario", path),
        `${path}.student_view_scenario`,
      ),
      active_student_course_membership: decodeStringEnum(
        field(record, "active_student_course_membership", path),
        `${path}.active_student_course_membership`,
        ["activeStudentCourseMembership"] as const,
      ),
      effective_assignment_policy: effective_assignment_policy(
        field(record, "effective_assignment_policy", path),
        `${path}.effective_assignment_policy`,
      ),
      student_feedback_release: disclosures(
        field(record, "student_feedback_release", path),
        `${path}.student_feedback_release`,
      ),
    };
  }
  if (kind === "denied") {
    requireOnlyFields(record, path, ["kind", "reason"]);
    return {
      kind,
      reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, [
        "activeStudentCourseMembershipRequired",
        "staleRevision",
      ] as const),
    };
  }
  throw new DecodeError(`${path}.kind`, "allowed or denied");
}

function accommodation(value: unknown, path: string): PreviewAccommodationComparison {
  const record = closed(value, path, ["before", "after"]);
  return {
    before: effective_assignment_policy(record.before, `${path}.before`),
    after: effective_assignment_policy(record.after, `${path}.after`),
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
    requireOnlyFields(record, path, [
      "kind",
      "membership",
      "display",
      "active_student_course_membership",
      "effective_assignment_policy",
    ]);
    return {
      kind,
      membership: reference(field(record, "membership", path), `${path}.membership`, "M"),
      display: label(field(record, "display", path), `${path}.display`),
      active_student_course_membership: decodeStringEnum(
        field(record, "active_student_course_membership", path),
        `${path}.active_student_course_membership`,
        ["activeStudentCourseMembership"] as const,
      ),
      effective_assignment_policy: effective_assignment_policy(
        field(record, "effective_assignment_policy", path),
        `${path}.effective_assignment_policy`,
      ),
    };
  }
  if (kind === "denied") {
    requireOnlyFields(record, path, ["kind", "membership", "display", "reason"]);
    return {
      kind,
      membership: reference(field(record, "membership", path), `${path}.membership`, "M"),
      display: label(field(record, "display", path), `${path}.display`),
      reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, [
        "noActiveStudentCourseMembership",
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

export function decodeStudentViewScenarioRequest(
  value: unknown,
  path = "request",
): StudentViewScenarioRequest {
  const record = closed(value, path, ["assignment", "revision", "selectedMoment", "modifiers"]);
  return {
    assignment: reference(record.assignment, `${path}.assignment`, "A"),
    revision: revision(record.revision, `${path}.revision`),
    selectedMoment: selectedMoment(record.selectedMoment, `${path}.selectedMoment`),
    modifiers: decodeSyntheticPreviewAccommodationAdjustmentRequest(
      record.modifiers,
      `${path}.modifiers`,
    ),
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
