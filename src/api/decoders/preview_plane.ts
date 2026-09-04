// Strict browser decoders for generated Instructor preview DTOs.

import { MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS } from "../../../generated/api/MAX_TEACHING_DISPLAY_LABEL_UNICODE_SCALARS";
import { MAX_TEACHING_PAGE_SIZE } from "../../../generated/api/MAX_TEACHING_PAGE_SIZE";
import type { HypotheticalStudentViewScenarioRequest } from "../../../generated/api/HypotheticalStudentViewScenarioRequest";
import type { InstructorPreviewSchedulePage } from "../../../generated/api/InstructorPreviewSchedulePage";
import type { InstructorPreviewScheduleRow } from "../../../generated/api/InstructorPreviewScheduleRow";
import type { PreviewAccommodationComparison } from "../../../generated/api/PreviewAccommodationComparison";
import type { StudentFeedbackReleaseView } from "../../../generated/api/StudentFeedbackReleaseView";
import type { PreviewEvaluation } from "../../../generated/api/PreviewEvaluation";
import type { PreviewPlaneResponse } from "../../../generated/api/PreviewPlaneResponse";
import type { EffectiveAssignmentPolicyView } from "../../../generated/api/EffectiveAssignmentPolicyView";
import type { PreviewSelectedMoment } from "../../../generated/api/PreviewSelectedMoment";
import type { StudentViewScenario } from "../../../generated/api/StudentViewScenario";
import type { SelectedStudentViewScenarioRequest } from "../../../generated/api/SelectedStudentViewScenarioRequest";
import type {
  QuestionPoolPreview,
  QuestionPoolPreviewItem,
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
import { decodeHypotheticalStudentViewScenarioModifiers } from "./teaching_operations";
import {
  decodeBoundedArray,
  decodeCursor,
  decodeIdentifier,
  field,
  requireOnlyFields,
} from "./shared";

const MAX_ROUTE_REFERENCE = 2_147_483_647;
const POLICY_SOURCES = ["base", "accommodation", "hypothetical_student_view_scenario"] as const;
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

function editNumber(value: unknown, path: string): string {
  const parsed = decodeString(value, path);
  if (!/^[1-9][0-9]{0,18}$/u.test(parsed) || BigInt(parsed) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(path, "a canonical positive PostgreSQL bigint Assignment edit number");
  }
  return parsed;
}

function previewItem(value: unknown, path: string): QuestionPoolPreviewItem {
  const record = closed(value, path, ["questionId", "questionTitle"]);
  return {
    questionId: questionId(record.questionId, `${path}.questionId`),
    questionTitle: label(record.questionTitle, `${path}.questionTitle`),
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
    "editNumber",
    "assignmentEntryId",
    "questionPoolLabel",
    "selectionCount",
    "selectionRule",
    "items",
    "selectedItems",
  ]);
  const items = decodeBoundedArray(
    record.items,
    `${path}.items`,
    MAX_TEACHING_PAGE_SIZE,
    previewItem,
  );
  const selectedItems = decodeBoundedArray(
    record.selectedItems,
    `${path}.selectedItems`,
    MAX_TEACHING_PAGE_SIZE,
    previewItem,
  );
  const selectionCount = decodeSafeInteger(record.selectionCount, `${path}.selectionCount`);
  const assignmentEntryId = decodeIdentifier(record.assignmentEntryId, `${path}.assignmentEntryId`);
  if (
    selectionCount < 1 ||
    selectionCount > items.length ||
    selectedItems.length !== selectionCount
  )
    throw new DecodeError(
      `${path}.selectionCount`,
      "a valid selection count for the returned pool",
    );
  const selectedIds = new Set(selectedItems.map((item) => item.questionId));
  if (
    selectedIds.size !== selectedItems.length ||
    !selectedItems.every((item) =>
      items.some((poolItem) => poolItem.questionId === item.questionId),
    )
  )
    throw new DecodeError(`${path}.selectedItems`, "unique Question Pool Item Question IDs");
  return {
    assignment: reference(record.assignment, `${path}.assignment`, "A"),
    editNumber: editNumber(record.editNumber, `${path}.editNumber`),
    assignmentEntryId,
    questionPoolLabel: label(record.questionPoolLabel, `${path}.questionPoolLabel`),
    selectionCount,
    selectionRule: selectionRule(record.selectionRule, `${path}.selectionRule`),
    items,
    selectedItems,
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
  const record = closed(value, path, ["value", "time_zone"]);
  const timeZone = label(record.time_zone, `${path}.time_zone`);
  if (Array.from(timeZone).length > 255)
    throw new DecodeError(`${path}.time_zone`, "a bounded IANA zone");
  return { value: courseLocalDateAndTime(record.value, `${path}.value`), time_zone: timeZone };
}

function assignmentPolicySourceKind(
  value: unknown,
  path: string,
): EffectiveAssignmentPolicyView["available_at"]["source"] {
  return decodeStringEnum(value, path, POLICY_SOURCES);
}

function timeField(value: unknown, path: string): EffectiveAssignmentPolicyView["available_at"] {
  const record = closed(value, path, ["value", "source"]);
  return {
    value: decodeNullable(record.value, `${path}.value`, courseLocalDateAndTime),
    source: assignmentPolicySourceKind(record.source, `${path}.source`),
  };
}

function limitField(
  value: unknown,
  path: string,
): EffectiveAssignmentPolicyView["assignment_attempt_time_limit_seconds"] {
  const record = closed(value, path, ["value", "source"]);
  const limit = decodeNullable(record.value, `${path}.value`, decodeSafeInteger);
  if (limit !== null && limit < 1)
    throw new DecodeError(`${path}.value`, "a positive safe integer");
  return { value: limit, source: assignmentPolicySourceKind(record.source, `${path}.source`) };
}

function effective_assignment_policy(value: unknown, path: string): EffectiveAssignmentPolicyView {
  const record = closed(value, path, [
    "available_at",
    "due_at",
    "closes_at",
    "assignment_attempt_time_limit_seconds",
    "attempt_limit",
    "late_work_rule",
    "assignment_deadline_rule",
  ]);
  const lateWorkRule = closed(record.late_work_rule, `${path}.late_work_rule`, ["value", "source"]);
  const assignmentDeadlineRule = closed(
    record.assignment_deadline_rule,
    `${path}.assignment_deadline_rule`,
    ["value", "source"],
  );
  return {
    available_at: timeField(record.available_at, `${path}.available_at`),
    due_at: timeField(record.due_at, `${path}.due_at`),
    closes_at: timeField(record.closes_at, `${path}.closes_at`),
    assignment_attempt_time_limit_seconds: limitField(
      record.assignment_attempt_time_limit_seconds,
      `${path}.assignment_attempt_time_limit_seconds`,
    ),
    attempt_limit: limitField(record.attempt_limit, `${path}.attempt_limit`),
    late_work_rule: {
      value: decodeStringEnum(lateWorkRule.value, `${path}.late_work_rule.value`, [
        "accept",
        "mark_late",
        "reject",
      ] as const),
      source: assignmentPolicySourceKind(lateWorkRule.source, `${path}.late_work_rule.source`),
    },
    assignment_deadline_rule: {
      value: decodeStringEnum(
        assignmentDeadlineRule.value,
        `${path}.assignment_deadline_rule.value`,
        ["auto_submit"] as const,
      ),
      source: assignmentPolicySourceKind(
        assignmentDeadlineRule.source,
        `${path}.assignment_deadline_rule.source`,
      ),
    },
  };
}

function studentViewScenario(value: unknown, path: string): StudentViewScenario {
  const record = closed(value, path, [
    "origin",
    "assignment",
    "edit_number",
    "selected_moment",
    "policy",
    "prior_assignment_attempt_count",
  ]);
  return {
    origin: decodeStringEnum(record.origin, `${path}.origin`, [
      "selected_student",
      "hypothetical",
    ] as const),
    assignment: reference(record.assignment, `${path}.assignment`, "A"),
    edit_number: editNumber(record.edit_number, `${path}.edit_number`),
    selected_moment: selectedMoment(record.selected_moment, `${path}.selected_moment`),
    policy: effective_assignment_policy(record.policy, `${path}.policy`),
    prior_assignment_attempt_count: nonnegativeInteger(
      record.prior_assignment_attempt_count,
      `${path}.prior_assignment_attempt_count`,
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
      "score_shown",
      "correctness_shown",
      "feedback_shown",
      "question_answer_shown",
      "question_answer_explanation_shown",
      "statistics_shown",
    ]);
    return {
      kind,
      moment: decodeStringEnum(field(record, "moment", path), `${path}.moment`, DISCLOSURE_MOMENTS),
      flags: {
        score_shown: decodeBoolean(flags.score_shown, `${path}.flags.score_shown`),
        correctness_shown: decodeBoolean(
          flags.correctness_shown,
          `${path}.flags.correctness_shown`,
        ),
        feedback_shown: decodeBoolean(flags.feedback_shown, `${path}.flags.feedback_shown`),
        question_answer_shown: decodeBoolean(
          flags.question_answer_shown,
          `${path}.flags.question_answer_shown`,
        ),
        question_answer_explanation_shown: decodeBoolean(
          flags.question_answer_explanation_shown,
          `${path}.flags.question_answer_explanation_shown`,
        ),
        statistics_shown: decodeBoolean(flags.statistics_shown, `${path}.flags.statistics_shown`),
      },
    };
  }
  if (kind === "unavailable") {
    requireOnlyFields(record, path, ["kind", "moment", "reason"]);
    return {
      kind,
      moment: decodeStringEnum(field(record, "moment", path), `${path}.moment`, DISCLOSURE_MOMENTS),
      reason: decodeStringEnum(field(record, "reason", path), `${path}.reason`, [
        "boundary_missing",
      ] as const),
    };
  }
  throw new DecodeError(`${path}.kind`, "a known Student Feedback Release View kind");
}

function disclosures(value: unknown, path: string): Array<StudentFeedbackReleaseView> {
  const decoded = decodeBoundedArray(
    value,
    path,
    DISCLOSURE_MOMENTS.length,
    student_feedback_release,
  );
  const moments = decoded.map((releaseView) => releaseView.moment);
  if (
    moments.length !== DISCLOSURE_MOMENTS.length ||
    moments.some((moment, index) => moment !== DISCLOSURE_MOMENTS[index])
  ) {
    throw new DecodeError(
      path,
      "one Student Feedback Release View for each disclosure moment in order: now, due, close",
    );
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
      "student_view_scenario_admission",
      "effective_assignment_policy",
      "student_feedback_release",
    ]);
    const decodedStudentViewScenario = studentViewScenario(
      field(record, "student_view_scenario", path),
      `${path}.student_view_scenario`,
    );
    const studentViewScenarioAdmission = decodeStringEnum(
      field(record, "student_view_scenario_admission", path),
      `${path}.student_view_scenario_admission`,
      [
        "selected_student_active_student_course_membership",
        "hypothetical_student_view_scenario_admission",
      ] as const,
    );
    const expectedAdmission =
      decodedStudentViewScenario.origin === "selected_student"
        ? "selected_student_active_student_course_membership"
        : "hypothetical_student_view_scenario_admission";
    if (studentViewScenarioAdmission !== expectedAdmission) {
      throw new DecodeError(
        `${path}.student_view_scenario_admission`,
        `${decodedStudentViewScenario.origin} Student View Scenario admission`,
      );
    }
    return {
      kind,
      student_view_scenario: decodedStudentViewScenario,
      student_view_scenario_admission: studentViewScenarioAdmission,
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
        "active_student_course_membership_required",
        "stale_revision",
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
        ["active_student_course_membership"] as const,
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
        "no_active_student_course_membership",
      ] as const),
    };
  }
  throw new DecodeError(`${path}.kind`, "granted or denied");
}

export function decodeInstructorPreviewSchedulePage(
  value: unknown,
  path = "response",
): InstructorPreviewSchedulePage {
  const record = closed(value, path, ["edit_number", "rows", "next_cursor"]);
  return {
    edit_number: editNumber(record.edit_number, `${path}.edit_number`),
    rows: decodeBoundedArray(record.rows, `${path}.rows`, MAX_TEACHING_PAGE_SIZE, scheduleRow),
    next_cursor: decodeNullable(record.next_cursor, `${path}.next_cursor`, decodeCursor),
  };
}

export function decodeHypotheticalStudentViewScenarioRequest(
  value: unknown,
  path = "request",
): HypotheticalStudentViewScenarioRequest {
  const record = closed(value, path, ["assignment", "edit_number", "selected_moment", "modifiers"]);
  return {
    assignment: reference(record.assignment, `${path}.assignment`, "A"),
    edit_number: editNumber(record.edit_number, `${path}.edit_number`),
    selected_moment: selectedMoment(record.selected_moment, `${path}.selected_moment`),
    modifiers: decodeHypotheticalStudentViewScenarioModifiers(
      record.modifiers,
      `${path}.modifiers`,
    ),
  };
}

export function decodeSelectedStudentViewScenarioRequest(
  value: unknown,
  path = "request",
): SelectedStudentViewScenarioRequest {
  const record = closed(value, path, [
    "assignment",
    "edit_number",
    "selected_moment",
    "selected_student_membership",
  ]);
  return {
    assignment: reference(record.assignment, `${path}.assignment`, "A"),
    edit_number: editNumber(record.edit_number, `${path}.edit_number`),
    selected_moment: selectedMoment(record.selected_moment, `${path}.selected_moment`),
    selected_student_membership: reference(
      record.selected_student_membership,
      `${path}.selected_student_membership`,
      "M",
    ),
  };
}
