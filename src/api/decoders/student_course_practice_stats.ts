// Strict decoder for self-only Course Practice Stats.

import type { StudentCoursePracticeStats } from "../student_course_practice_stats";
import {
  DecodeError,
  decodeArray,
  decodeFiniteNumber,
  decodeNonnegativeInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import { parseAssessmentAttemptId } from "../../navigation/public_route";
import { validateCanonicalQuestionIdSyntax } from "../../question_id";
import { field, requireOnlyFields } from "./shared";

function questionStats(
  value: unknown,
  path: string,
): StudentCoursePracticeStats["questions"][number] {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "publishedQuestionRevisionTuple",
    "fullCreditAttemptCount",
    "partialCreditAttemptCount",
    "incorrectAttemptCount",
    "unansweredAttemptCount",
    "disclosedAttemptCount",
    "notFullCreditCount",
    "averageDisplayDurationMs",
    "displayDurationSampleCount",
    "relevantAssessmentAttemptId",
  ]);
  const rawTuple = decodeRecord(
    field(record, "publishedQuestionRevisionTuple", path),
    `${path}.publishedQuestionRevisionTuple`,
  );
  requireOnlyFields(rawTuple, `${path}.publishedQuestionRevisionTuple`, [
    "publishedQuestionId",
    "revisionNumber",
  ]);
  const publishedQuestionId = decodeString(
    field(rawTuple, "publishedQuestionId", `${path}.publishedQuestionRevisionTuple`),
    `${path}.publishedQuestionRevisionTuple.publishedQuestionId`,
  );
  if (validateCanonicalQuestionIdSyntax(publishedQuestionId) === null) {
    throw new DecodeError(
      `${path}.publishedQuestionRevisionTuple.publishedQuestionId`,
      "a canonical Published Question ID",
    );
  }
  const revisionNumber = decodeNonnegativeInteger(
    field(rawTuple, "revisionNumber", `${path}.publishedQuestionRevisionTuple`),
    `${path}.publishedQuestionRevisionTuple.revisionNumber`,
  );
  if (revisionNumber === 0) {
    throw new DecodeError(
      `${path}.publishedQuestionRevisionTuple.revisionNumber`,
      "a positive revision number",
    );
  }
  const fullCreditAttemptCount = decodeNonnegativeInteger(
    field(record, "fullCreditAttemptCount", path),
    `${path}.fullCreditAttemptCount`,
  );
  const partialCreditAttemptCount = decodeNonnegativeInteger(
    field(record, "partialCreditAttemptCount", path),
    `${path}.partialCreditAttemptCount`,
  );
  const incorrectAttemptCount = decodeNonnegativeInteger(
    field(record, "incorrectAttemptCount", path),
    `${path}.incorrectAttemptCount`,
  );
  const unansweredAttemptCount = decodeNonnegativeInteger(
    field(record, "unansweredAttemptCount", path),
    `${path}.unansweredAttemptCount`,
  );
  const disclosedAttemptCount = decodeNonnegativeInteger(
    field(record, "disclosedAttemptCount", path),
    `${path}.disclosedAttemptCount`,
  );
  const notFullCreditCount = decodeNonnegativeInteger(
    field(record, "notFullCreditCount", path),
    `${path}.notFullCreditCount`,
  );
  const averageValue = field(record, "averageDisplayDurationMs", path);
  const averageDisplayDurationMs =
    averageValue === null
      ? null
      : decodeFiniteNumber(averageValue, `${path}.averageDisplayDurationMs`);
  const displayDurationSampleCount = decodeNonnegativeInteger(
    field(record, "displayDurationSampleCount", path),
    `${path}.displayDurationSampleCount`,
  );
  const rawAttemptId = decodeString(
    field(record, "relevantAssessmentAttemptId", path),
    `${path}.relevantAssessmentAttemptId`,
  );
  const relevantAssessmentAttemptId = parseAssessmentAttemptId(rawAttemptId);
  if (relevantAssessmentAttemptId === null) {
    throw new DecodeError(`${path}.relevantAssessmentAttemptId`, "an Assessment Attempt UUID");
  }
  if (
    fullCreditAttemptCount +
      partialCreditAttemptCount +
      incorrectAttemptCount +
      unansweredAttemptCount !==
      disclosedAttemptCount ||
    partialCreditAttemptCount + incorrectAttemptCount + unansweredAttemptCount !==
      notFullCreditCount ||
    disclosedAttemptCount === 0 ||
    (averageDisplayDurationMs !== null && averageDisplayDurationMs < 0) ||
    displayDurationSampleCount > disclosedAttemptCount ||
    (displayDurationSampleCount === 0) !== (averageDisplayDurationMs === null)
  ) {
    throw new DecodeError(path, "consistent disclosed Practice Stats counts");
  }
  return {
    publishedQuestionRevisionTuple: { publishedQuestionId, revisionNumber },
    fullCreditAttemptCount,
    partialCreditAttemptCount,
    incorrectAttemptCount,
    unansweredAttemptCount,
    disclosedAttemptCount,
    notFullCreditCount,
    averageDisplayDurationMs,
    displayDurationSampleCount,
    relevantAssessmentAttemptId,
  };
}

export function decodeStudentCoursePracticeStats(
  value: unknown,
  path = "response",
): StudentCoursePracticeStats {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["questions"]);
  const questions = decodeArray(
    field(record, "questions", path),
    `${path}.questions`,
    questionStats,
  );
  if (
    questions.some((question, index) => {
      const previous = questions[index - 1];
      return (
        previous !== undefined &&
        (question.notFullCreditCount > previous.notFullCreditCount ||
          (question.notFullCreditCount === previous.notFullCreditCount &&
            question.disclosedAttemptCount > previous.disclosedAttemptCount))
      );
    })
  ) {
    throw new DecodeError(`${path}.questions`, "Practice Stats ranked by not-full-credit count");
  }
  return { questions };
}
