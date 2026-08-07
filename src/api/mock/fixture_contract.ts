// fixture_contract.ts - browser-safe contract for the WP-C7 typed fixture.

import type { AssetId } from "../../../generated/api/AssetId";
import type { AssignmentEnrollment } from "../../../generated/api/AssignmentEnrollment";
import type { AssignmentRun } from "../../../generated/api/AssignmentRun";
import type { ObjectId } from "../../../generated/api/ObjectId";
import type { QuestionAttempt } from "../../../generated/api/QuestionAttempt";
import type { QuestionDefinition } from "../../../generated/api/QuestionDefinition";
import type { StudentAssignmentSummary } from "../../../generated/api/StudentAssignmentSummary";
import type { AssignmentSummary, CourseSummary } from "../contracts";

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
  readonly fixtureSchemaVersion: 1;
  readonly modelSchemaVersion: 1;
  readonly publishedProblem: QuestionDefinition;
  readonly draft: QuestionDefinition;
  readonly assets: ReadonlyArray<MockFixtureAsset>;
  readonly course: CourseSummary;
  readonly assignment: AssignmentSummary;
  readonly enrollment: AssignmentEnrollment;
  readonly runs: ReadonlyArray<AssignmentRun>;
  readonly attempts: ReadonlyArray<QuestionAttempt>;
  readonly summary: StudentAssignmentSummary;
}
