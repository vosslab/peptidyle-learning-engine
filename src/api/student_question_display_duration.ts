// Browser contract for cumulative Question display-duration checkpoints.

import type { AssessmentAttemptId } from "../../generated/api/AssessmentAttemptId";
import type { StudentQuestionDisplayDurationCheckpoint as Contract } from "../../generated/api/StudentQuestionDisplayDurationCheckpoint";

export type StudentQuestionDisplayDurationCheckpoint = Contract;

export interface StudentQuestionDisplayDurationClient {
  readonly checkpointStudentQuestionDisplayDuration: (
    assessmentAttemptId: AssessmentAttemptId,
    position: number,
    cumulativeDisplayDurationMs: number,
  ) => Promise<StudentQuestionDisplayDurationCheckpoint>;
}
