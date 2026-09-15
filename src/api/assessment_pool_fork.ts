// Browser contract for Assessment-owned immutable Question Pool fork operations.

import type { AssessmentEditNumber } from "../../generated/api/AssessmentEditNumber";
import type { AssessmentEntryId } from "../../generated/api/AssessmentEntryId";
import type { AssessmentQuestionPoolForkView } from "../../generated/api/AssessmentQuestionPoolForkView";
import type { AssessmentQuestionPoolSelectionCountReceipt } from "../../generated/api/AssessmentQuestionPoolSelectionCountReceipt";
import type { AssessmentReference } from "../../generated/api/AssessmentReference";
import type { AssessmentPointValue } from "../../generated/api/AssessmentPointValue";
import type { AssessmentEntryScoringRule } from "../../generated/api/AssessmentEntryScoringRule";
import type { CourseInstanceReference } from "../../generated/api/CourseInstanceReference";
import type { QuestionId } from "../../generated/api/QuestionId";
import type { QuestionPoolSelectedQuestionOrder } from "../../generated/api/QuestionPoolSelectedQuestionOrder";
import type { QuestionRevisionReference } from "../../generated/api/QuestionRevisionReference";

/** Closed browser input for importing one reusable Pool into an Assessment-owned fork. */
export interface ImportAssessmentQuestionPoolForkInput {
  readonly sourceQuestionPoolId: QuestionId;
  readonly authoredPosition: number;
  readonly selectionCount: number;
  readonly pointsPerItem: AssessmentPointValue;
  readonly selectedQuestionOrder: QuestionPoolSelectedQuestionOrder;
  readonly scoringRule: AssessmentEntryScoringRule;
}

/** Closed browser input for appending one immutable revision to an Assessment-owned Pool fork. */
export interface AppendAssessmentQuestionPoolForkRevisionInput {
  readonly expectedPoolMetadataEtag: string;
  readonly members: ReadonlyArray<QuestionRevisionReference>;
  readonly interchangeabilityAttested: boolean;
}

/** Whole receipt after appending a new immutable Assessment-owned Pool fork Revision. */
export interface AppendedAssessmentQuestionPoolForkRevision {
  readonly assessmentEntryId: AssessmentEntryId;
  readonly revisionNumber: number;
  readonly metadataEtag: string;
  readonly assessmentEditNumber: AssessmentEditNumber;
}

/** Server-issued Assessment entry and fork Revision 1 after an atomic Pool import. */
export interface ImportedAssessmentQuestionPoolFork {
  readonly assessmentEntryId: AssessmentEntryId;
  readonly questionPoolId: QuestionId;
  readonly revisionNumber: number;
  readonly assessmentEditNumber: AssessmentEditNumber;
}

/** Narrow Assessment Pool-fork capability; it neither saves generic Assessment content nor issues Pool IDs. */
export interface AssessmentPoolForkClient {
  readonly getAssessmentQuestionPoolFork: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    entry: AssessmentEntryId,
  ) => Promise<AssessmentQuestionPoolForkView>;
  readonly importAssessmentQuestionPoolFork: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    input: ImportAssessmentQuestionPoolForkInput,
    etag: string,
  ) => Promise<ImportedAssessmentQuestionPoolFork>;
  readonly appendAssessmentQuestionPoolForkRevision: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    entry: AssessmentEntryId,
    input: AppendAssessmentQuestionPoolForkRevisionInput,
    etag: string,
  ) => Promise<AppendedAssessmentQuestionPoolForkRevision>;
  readonly updateAssessmentQuestionPoolSelectionCount: (
    course: CourseInstanceReference,
    assessment: AssessmentReference,
    entry: AssessmentEntryId,
    selectionCount: number,
    etag: string,
  ) => Promise<AssessmentQuestionPoolSelectionCountReceipt>;
}
