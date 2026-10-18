import type { StudentAssessmentAttemptHistoryQuestion } from "../api/assessment_attempt_history";
import type { RecordField } from "../components/data_sheet/record_fields";
import type { RecordAction, RecordContent } from "../components/data_sheet/record_content";
import { formatPointScore } from "../score_format";

type RecordedQuestion = StudentAssessmentAttemptHistoryQuestion;
type SortKey = "position" | "response" | "correctness" | "score";
type FilterKey = "response" | "correctness";
export type RecordedQuestionReview = "recorded" | "answer";

export function recordedResponseLabel(question: RecordedQuestion): string {
  return question.responseState === "closed" ? "Unanswered" : "Submitted";
}

export function recordedCorrectnessLabel(question: RecordedQuestion): string {
  if (question.correctness === undefined) return "Not available";
  return question.correctness ? "Marked correct" : "Marked not correct";
}

export function recordedScoreLabel(question: RecordedQuestion): string {
  return question.pointsEarned === undefined || question.pointsPossible === undefined
    ? "Not available"
    : `${formatPointScore(question.pointsEarned, question.pointsPossible)} points`;
}

export const assessmentAttemptRecordFields: ReadonlyArray<
  RecordField<RecordedQuestion, SortKey, FilterKey>
> = [
  {
    id: "question",
    label: "Question",
    role: "identity",
    sortKey: "position",
    value: (question) => `Question ${question.position}`,
    defaultColumn: { priority: 0, width: "12rem" },
  },
  {
    id: "response",
    label: "Response",
    sortKey: "response",
    filterKey: "response",
    value: recordedResponseLabel,
    defaultColumn: { priority: 1, width: "10rem" },
  },
  {
    id: "correctness",
    label: "Result",
    sortKey: "correctness",
    filterKey: "correctness",
    value: recordedCorrectnessLabel,
    defaultColumn: { priority: 2, width: "12rem" },
  },
  {
    id: "score",
    label: "Points",
    sortKey: "score",
    value: (question) => question.pointsEarned ?? null,
    render: recordedScoreLabel,
    defaultColumn: { priority: 3, width: "12rem" },
  },
];

export function compareAssessmentAttemptRecords(
  key: SortKey,
  left: RecordedQuestion,
  right: RecordedQuestion,
): number | undefined {
  return key === "position" ? left.position - right.position : undefined;
}

/** Index only the server-released result facts, never undisclosed answer content. */
export function assessmentAttemptRecordKeyword(question: RecordedQuestion): string {
  return `Question ${question.position} ${recordedResponseLabel(question)} ${recordedCorrectnessLabel(question)} ${recordedScoreLabel(question)}`;
}

export function assessmentAttemptRecordContent(
  question: RecordedQuestion,
  retry: (review: RecordedQuestionReview) => void,
): RecordContent {
  const actions: RecordAction[] = [];
  if (question.backendReview === "available") {
    actions.push({
      id: "retry_recorded_question",
      kind: "command",
      label: "Retry recorded Question",
      onClick: () => retry("recorded"),
    });
  }
  if (question.backendAnswerReview === "available") {
    actions.push({
      id: "retry_correct_answer",
      kind: "command",
      label: "Retry correct answer",
      onClick: () => retry("answer"),
    });
  }
  return {
    title: `Question ${question.position}`,
    details: [
      { kind: "text", label: "Response", value: recordedResponseLabel(question) },
      { kind: "text", label: "Result", value: recordedCorrectnessLabel(question) },
      { kind: "text", label: "Points", value: recordedScoreLabel(question) },
    ],
    actions,
  };
}
