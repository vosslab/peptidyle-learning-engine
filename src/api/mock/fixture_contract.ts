// fixture_contract.ts - browser-safe contract for the WP-C7 typed fixture.

import type { AssetId } from "../../../generated/api/AssetId";
import type { AssignmentEnrollment } from "../../../generated/api/AssignmentEnrollment";
import type { AssignmentRun } from "../../../generated/api/AssignmentRun";
import type { CatalogProblemSummary } from "../../../generated/api/CatalogProblemSummary";
import type { ObjectId } from "../../../generated/api/ObjectId";
import type { QuestionAttempt } from "../../../generated/api/QuestionAttempt";
import type { DraftQuestionDefinition } from "../../../generated/api/DraftQuestionDefinition";
import type { GradebookSummaryRow } from "../../../generated/api/GradebookSummaryRow";
import type { QuestionDefinition } from "../../../generated/api/QuestionDefinition";
import type { StudentAssignmentSummary } from "../../../generated/api/StudentAssignmentSummary";
import type { LearnerAssignmentProgress } from "../../../generated/api/LearnerAssignmentProgress";
import type { AssignmentSummary, CourseSummary } from "../contracts";
import type { LearnerAssignmentSummary } from "../../../generated/api/LearnerAssignmentSummary";

/** One browser-loadable asset belonging to the published fixture version. */
export interface MockFixtureAsset {
  readonly id: AssetId;
  readonly object: ObjectId;
  readonly filename: string;
  readonly mediaType: string;
  readonly sha256: string;
}

/** Complete typed data set shared by every WP-C7 mock route group. */
export interface MockFixtureCorpus {
  readonly fixtureSchemaVersion: 4;
  readonly modelSchemaVersion: 1;
  readonly catalogProblem: CatalogProblemSummary;
  readonly publishedProblem: QuestionDefinition;
  readonly draft: DraftQuestionDefinition;
  readonly assets: ReadonlyArray<MockFixtureAsset>;
  readonly course: CourseSummary;
  readonly assignment: AssignmentSummary;
  readonly enrollment: AssignmentEnrollment;
  readonly runs: ReadonlyArray<AssignmentRun>;
  readonly attempts: ReadonlyArray<QuestionAttempt>;
  readonly summary: StudentAssignmentSummary;
  readonly gradebook: ReadonlyArray<GradebookSummaryRow>;
}

/**
 * Mock-only server projection for learner routes.  It intentionally copies no
 * storage identifiers and lets focused UI tests exercise every presentation
 * state by substituting this record at their transport boundary.
 */
export function fixtureLearnerProgress(
  summary: StudentAssignmentSummary,
): LearnerAssignmentProgress {
  const hasActivity = summary.completedRunCount > 0 || summary.totalQuestionAttempts > 0;
  if (!hasActivity) {
    return {
      scoreState: "noActivity",
      scoringStatus: "current",
      currentScore: null,
      bestScore: null,
      latestScore: null,
      completedRunCount: 0,
      totalQuestionAttempts: 0,
      lastActivityAt: null,
    };
  }
  return {
    scoreState: "available",
    scoringStatus: "current",
    currentScore: summary.currentScore,
    bestScore: summary.bestScore,
    latestScore: summary.latestScore,
    completedRunCount: summary.completedRunCount,
    totalQuestionAttempts: summary.totalQuestionAttempts,
    lastActivityAt: summary.lastActivityAt,
  };
}

/** Remove assignment authority inputs before a mock serves a learner route. */
export function fixtureLearnerAssignment(assignment: AssignmentSummary): LearnerAssignmentSummary {
  return {
    id: assignment.id,
    reference: assignment.reference,
    title: assignment.title,
  };
}
