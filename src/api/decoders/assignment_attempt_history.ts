// Strict decoder for the selected Student Assignment Attempt history route.

import type { StudentAssignmentAttemptHistory } from "../assignment_attempt_history";
import {
  DecodeError,
  decodeArray,
  decodeFiniteNumber,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import {
  decodeAssignmentReference,
  decodeAssignmentTitle,
  decodeCourseInstanceReference,
  decodeCourseName,
  field,
  requireOnlyFields,
} from "./shared";
import { COURSE_THEME_VALUES } from "../../../generated/api/CourseTheme";
import { decodeStringEnum } from "../decoder";
import { parseAssignmentAttemptReference } from "../../navigation/public_route";
import { decodeQuestionContentBlock } from "./question_response_format";
import { decodeStudentFeedback } from "./question_delivery";

function state(value: unknown, path: string): "submitted" | "closed" {
  const decoded = decodeString(value, path);
  if (decoded !== "submitted" && decoded !== "closed") {
    throw new DecodeError(path, "a completed Assignment Attempt state");
  }
  return decoded;
}

export function decodeStudentAssignmentAttemptHistory(
  value: unknown,
  path = "response",
): StudentAssignmentAttemptHistory {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "assignmentAttempt",
    "attemptNumber",
    "course",
    "assignment",
    "state",
    "score",
    "questions",
  ]);
  const reference = field(record, "assignmentAttempt", path);
  if (typeof reference !== "string")
    throw new DecodeError(`${path}.assignmentAttempt`, "an Assignment Attempt R- reference");
  const assignmentAttempt = parseAssignmentAttemptReference(reference);
  if (assignmentAttempt === null)
    throw new DecodeError(`${path}.assignmentAttempt`, "an Assignment Attempt R- reference");
  const assignment = decodeRecord(field(record, "assignment", path), `${path}.assignment`);
  requireOnlyFields(assignment, `${path}.assignment`, ["reference", "title"]);
  const course = decodeRecord(field(record, "course", path), `${path}.course`);
  requireOnlyFields(course, `${path}.course`, ["reference", "shortName", "longName", "theme"]);
  const score = optionalNestedScore(record.score, `${path}.score`);
  const decoded = {
    assignmentAttempt,
    attemptNumber: decodePositiveInteger(
      field(record, "attemptNumber", path),
      `${path}.attemptNumber`,
    ),
    course: {
      reference: decodeCourseInstanceReference(
        field(course, "reference", `${path}.course`),
        `${path}.course.reference`,
      ),
      shortName: decodeCourseName(
        field(course, "shortName", `${path}.course`),
        `${path}.course.shortName`,
      ),
      longName: decodeCourseName(
        field(course, "longName", `${path}.course`),
        `${path}.course.longName`,
      ),
      theme: decodeStringEnum(
        field(course, "theme", `${path}.course`),
        `${path}.course.theme`,
        COURSE_THEME_VALUES,
      ),
    },
    assignment: {
      reference: decodeAssignmentReference(
        field(assignment, "reference", `${path}.assignment`),
        `${path}.assignment.reference`,
      ),
      title: decodeAssignmentTitle(
        field(assignment, "title", `${path}.assignment`),
        `${path}.assignment.title`,
      ),
    },
    state: state(field(record, "state", path), `${path}.state`),
    ...(score === undefined ? {} : { score }),
    questions: decodeArray(
      field(record, "questions", path),
      `${path}.questions`,
      (question, questionPath) => {
        const item = decodeRecord(question, questionPath);
        requireOnlyFields(item, questionPath, [
          "position",
          "responseState",
          "response",
          "correctness",
          "pointsEarned",
          "pointsPossible",
          "choiceFeedback",
          "correctFeedback",
          "incorrectFeedback",
          "questionAnswer",
          "questionAnswerExplanation",
        ]);
        optionalScore(item, questionPath);
        const feedback = decodeStudentFeedback(
          {
            ...(item.correctness === undefined ? {} : { correctness: item.correctness }),
            ...(item.pointsEarned === undefined ? {} : { pointsEarned: item.pointsEarned }),
            ...(item.pointsPossible === undefined ? {} : { pointsPossible: item.pointsPossible }),
            ...(item.choiceFeedback === undefined ? {} : { choiceFeedback: item.choiceFeedback }),
            ...(item.correctFeedback === undefined
              ? {}
              : { correctFeedback: item.correctFeedback }),
            ...(item.incorrectFeedback === undefined
              ? {}
              : { incorrectFeedback: item.incorrectFeedback }),
            ...(item.questionAnswer === undefined ? {} : { questionAnswer: item.questionAnswer }),
            ...(item.questionAnswerExplanation === undefined
              ? {}
              : { questionAnswerExplanation: item.questionAnswerExplanation }),
          },
          questionPath,
        );
        const response = item.response;
        return {
          position: decodePositiveInteger(
            field(item, "position", questionPath),
            `${questionPath}.position`,
          ),
          responseState: state(
            field(item, "responseState", questionPath),
            `${questionPath}.responseState`,
          ),
          ...feedback,
          ...(response === undefined
            ? {}
            : {
                response: decodeArray(response, `${questionPath}.response`, (block, blockPath) =>
                  decodeQuestionContentBlock(block, blockPath, true),
                ),
              }),
        };
      },
    ),
  };
  return decoded;
}

function optionalNestedScore(
  value: unknown,
  path: string,
): { readonly pointsEarned: number; readonly pointsPossible: number } | undefined {
  if (value === undefined) return undefined;
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["pointsEarned", "pointsPossible"]);
  return optionalScore(record, path);
}

function optionalScore(
  record: Record<string, unknown>,
  path: string,
): { readonly pointsEarned: number; readonly pointsPossible: number } | undefined {
  const earned = record.pointsEarned;
  const possible = record.pointsPossible;
  if (earned === undefined && possible === undefined) return undefined;
  if (earned === undefined || possible === undefined) {
    throw new DecodeError(path, "a complete disclosed score");
  }
  const pointsEarned = decodeFiniteNumber(earned, `${path}.pointsEarned`);
  const pointsPossible = decodeFiniteNumber(possible, `${path}.pointsPossible`);
  if (pointsEarned < 0 || pointsPossible < 0 || pointsEarned > pointsPossible) {
    throw new DecodeError(path, "an ordered nonnegative score");
  }
  return { pointsEarned, pointsPossible };
}
