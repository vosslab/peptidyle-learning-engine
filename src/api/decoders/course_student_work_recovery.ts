import type {
  RecoveredAttempt,
  RecoveredQuestion,
  RecoverySelection,
  RecoverySummary,
} from "../course_student_work_recovery";
import {
  DecodeError,
  decodeArray,
  decodeNullable,
  decodePositiveInteger,
  decodeNonnegativeInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import {
  field,
  requireOnlyFields,
  decodeCourseInstanceId,
  decodeAssessmentId,
  decodeQuestionRevisionTuple,
} from "./shared";
import { parseAssessmentAttemptId } from "../../navigation/public_route";

const commonFields = [
  "courseInstanceId",
  "rosterId",
  "assessmentId",
  "assessmentTitle",
  "assessmentAttemptId",
  "assessmentAttemptNumber",
  "startedAt",
  "studentDataArchivedAt",
  "deleteDueAt",
];
const questionTextFields = [
  "poolText",
  "attemptText",
  "presentationText",
  "reproductionText",
  "backendDocumentText",
  "savedResponseText",
  "finalizedResponseText",
  "gradingText",
] as const;
function nullableText(value: unknown, path: string): string | null {
  return decodeNullable(value, path, decodeString);
}
function common(
  record: Record<string, unknown>,
  path: string,
): Omit<RecoverySummary, "submittedAt"> {
  const text = (key: string): string => decodeString(field(record, key, path), `${path}.${key}`);
  const assessmentAttemptId = text("assessmentAttemptId");
  if (parseAssessmentAttemptId(assessmentAttemptId) === null)
    throw new DecodeError(`${path}.assessmentAttemptId`, "an Assessment Attempt UUID");
  return {
    courseInstanceId: decodeCourseInstanceId(
      field(record, "courseInstanceId", path),
      `${path}.courseInstanceId`,
    ),
    rosterId: nullableText(field(record, "rosterId", path), `${path}.rosterId`),
    assessmentId: decodeAssessmentId(field(record, "assessmentId", path), `${path}.assessmentId`),
    assessmentTitle: text("assessmentTitle"),
    assessmentAttemptId,
    assessmentAttemptNumber: decodePositiveInteger(
      field(record, "assessmentAttemptNumber", path),
      `${path}.assessmentAttemptNumber`,
    ),
    startedAt: text("startedAt"),
    studentDataArchivedAt: text("studentDataArchivedAt"),
    deleteDueAt: text("deleteDueAt"),
  };
}
function summary(value: unknown, path: string): RecoverySummary {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [...commonFields, "submittedAt"]);
  return {
    ...common(record, path),
    submittedAt: nullableText(field(record, "submittedAt", path), `${path}.submittedAt`),
  };
}
function question(value: unknown, path: string): RecoveredQuestion {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "issuedPosition",
    "questionRevisionTuple",
    "deliveryText",
    "unavailableEvidence",
    ...questionTextFields,
  ]);
  const texts = Object.fromEntries(
    questionTextFields.map((key) => [
      key,
      nullableText(field(record, key, path), `${path}.${key}`),
    ]),
  ) as Pick<RecoveredQuestion, (typeof questionTextFields)[number]>;
  return {
    issuedPosition: decodeNonnegativeInteger(
      field(record, "issuedPosition", path),
      `${path}.issuedPosition`,
    ),
    questionRevisionTuple: decodeQuestionRevisionTuple(
      field(record, "questionRevisionTuple", path),
      `${path}.questionRevisionTuple`,
    ),
    deliveryText: decodeString(field(record, "deliveryText", path), `${path}.deliveryText`),
    ...texts,
    unavailableEvidence: decodeArray(
      field(record, "unavailableEvidence", path),
      `${path}.unavailableEvidence`,
      decodeString,
    ),
  };
}
// ASVS 1.5.2: closed objects; retained evidence stays text, never executable markup.
export function decodeRecoverySelection(value: unknown, path = "response"): RecoverySelection {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["action", "courseInstanceId", "attempts", "nextCursor"]);
  if (field(record, "action", path) !== "select") throw new DecodeError(`${path}.action`, "select");
  return {
    action: "select",
    courseInstanceId: decodeCourseInstanceId(
      field(record, "courseInstanceId", path),
      `${path}.courseInstanceId`,
    ),
    attempts: decodeArray(field(record, "attempts", path), `${path}.attempts`, summary),
    nextCursor: nullableText(field(record, "nextCursor", path), `${path}.nextCursor`),
  };
}
export function decodeRecoveredAttempt(value: unknown, path = "response"): RecoveredAttempt {
  const response = decodeRecord(value, path);
  requireOnlyFields(response, path, ["action", "attempt"]);
  if (field(response, "action", path) !== "recover")
    throw new DecodeError(`${path}.action`, "recover");
  const record = decodeRecord(field(response, "attempt", path), `${path}.attempt`);
  path = `${path}.attempt`;
  requireOnlyFields(record, path, [
    ...commonFields,
    "expiresAt",
    "attemptFactsText",
    "submissionText",
    "questions",
  ]);
  return {
    ...common(record, path),
    expiresAt: nullableText(field(record, "expiresAt", path), `${path}.expiresAt`),
    attemptFactsText: decodeString(
      field(record, "attemptFactsText", path),
      `${path}.attemptFactsText`,
    ),
    submissionText: nullableText(field(record, "submissionText", path), `${path}.submissionText`),
    questions: decodeArray(field(record, "questions", path), `${path}.questions`, question),
  };
}
