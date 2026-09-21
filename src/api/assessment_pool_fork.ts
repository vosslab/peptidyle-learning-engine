// Browser contract for Assessment-owned immutable Question Pool fork operations.

import type { AssessmentEditNumber } from "../../generated/api/AssessmentEditNumber";
import type { AssessmentEntryId } from "../../generated/api/AssessmentEntryId";
import type { AssessmentQuestionPoolForkView } from "../../generated/api/AssessmentQuestionPoolForkView";
import type { AssessmentQuestionPoolSelectionCountReceipt } from "../../generated/api/AssessmentQuestionPoolSelectionCountReceipt";
import type { AssessmentId } from "../../generated/api/AssessmentId";
import type { AssessmentPointValue } from "../../generated/api/AssessmentPointValue";
import type { AssessmentEntryScoringRule } from "../../generated/api/AssessmentEntryScoringRule";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { QuestionPoolId } from "../../generated/api/QuestionPoolId";
import type { QuestionPoolSelectedQuestionOrder } from "../../generated/api/QuestionPoolSelectedQuestionOrder";
import type { QuestionPoolEditNumber } from "../../generated/api/QuestionPoolEditNumber";
import type { PublishedQuestionRevisionTuple } from "../../generated/api/PublishedQuestionRevisionTuple";

/** Closed browser input for importing one reusable Pool into an Assessment-owned fork. */
export interface ImportAssessmentQuestionPoolForkInput {
  readonly sourceQuestionPoolId: QuestionPoolId;
  readonly authoredPosition: number;
  readonly selectionCount: number;
  readonly pointsPerItem: AssessmentPointValue;
  readonly selectedQuestionOrder: QuestionPoolSelectedQuestionOrder;
  readonly scoringRule: AssessmentEntryScoringRule;
}

/** Closed browser input for replacing current membership of an Assessment-owned Pool fork. */
export interface AppendAssessmentQuestionPoolForkMembersInput {
  readonly expectedQuestionPoolEditNumber: QuestionPoolEditNumber;
  readonly members: ReadonlyArray<PublishedQuestionRevisionTuple>;
  readonly interchangeabilityAttested: boolean;
}

/** Whole receipt after saving current membership of an Assessment-owned Pool fork. */
export interface AppendedAssessmentQuestionPoolForkMembers {
  readonly assessmentEntryId: AssessmentEntryId;
  readonly questionPoolEditNumber: QuestionPoolEditNumber;
  readonly assessmentEditNumber: AssessmentEditNumber;
}

/** Server-issued Assessment entry and fork Pool at Edit Number 1 after an atomic Pool import. */
export interface ImportedAssessmentQuestionPoolFork {
  readonly assessmentEntryId: AssessmentEntryId;
  readonly questionPoolId: QuestionPoolId;
  readonly questionPoolEditNumber: QuestionPoolEditNumber;
  readonly assessmentEditNumber: AssessmentEditNumber;
}

/** Narrow Assessment Pool-fork capability; it neither saves generic Assessment content nor issues Pool IDs. */
export interface AssessmentPoolForkClient {
  readonly getAssessmentQuestionPoolFork: (
    courseInstanceId: CourseInstanceId,
    assessmentId: AssessmentId,
    assessmentEntryId: AssessmentEntryId,
  ) => Promise<AssessmentQuestionPoolForkView>;
  readonly importAssessmentQuestionPoolFork: (
    courseInstanceId: CourseInstanceId,
    assessmentId: AssessmentId,
    input: ImportAssessmentQuestionPoolForkInput,
    expectedAssessmentEditNumber: AssessmentEditNumber,
  ) => Promise<ImportedAssessmentQuestionPoolFork>;
  readonly appendAssessmentQuestionPoolForkMembers: (
    courseInstanceId: CourseInstanceId,
    assessmentId: AssessmentId,
    assessmentEntryId: AssessmentEntryId,
    input: AppendAssessmentQuestionPoolForkMembersInput,
    expectedAssessmentEditNumber: AssessmentEditNumber,
  ) => Promise<AppendedAssessmentQuestionPoolForkMembers>;
  readonly updateAssessmentQuestionPoolSelectionCount: (
    courseInstanceId: CourseInstanceId,
    assessmentId: AssessmentId,
    assessmentEntryId: AssessmentEntryId,
    selectionCount: number,
    expectedAssessmentEditNumber: AssessmentEditNumber,
  ) => Promise<AssessmentQuestionPoolSelectionCountReceipt>;
}
