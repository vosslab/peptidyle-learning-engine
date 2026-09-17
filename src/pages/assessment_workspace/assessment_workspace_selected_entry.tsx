// Compact, scan-friendly Assessment Entry presentation for the Question editor.

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { QuestionRevisionReference } from "../../../generated/api/QuestionRevisionReference";
import type { BloomClassificationView } from "../../../generated/api/BloomClassificationView";
import type { JSX } from "solid-js";

function questionAttemptLimitLabel(entry: AssessmentEntry): string {
  const limit = entry.questionAttemptLimit.maxAttempts;
  if (limit === null) return "Unlimited";
  return `${limit} ${limit === 1 ? "attempt" : "attempts"}`;
}

function questionTimeLimitLabel(entry: AssessmentEntry): string {
  if (entry.questionAttemptTimeLimit.kind === "unlimited") return "No time limit";
  const { seconds, graceSeconds } = entry.questionAttemptTimeLimit;
  return `${seconds}s limit; ${graceSeconds}s grace`;
}

function assessmentEntryAvailabilityLabel(availability: AssessmentEntry["availability"]): string {
  return availability === "available" ? "Available" : "Retired";
}

function assessmentEntryScoringRuleLabel(scoringRule: AssessmentEntry["scoringRule"]): string {
  switch (scoringRule) {
    case "normal":
      return "Normal";
    case "fullCredit":
      return "Full credit";
    case "extraCredit":
      return "Extra credit";
    case "excluded":
      return "Excluded";
  }
}

export interface SelectedAssessmentEntryIdentityProps {
  readonly entry: AssessmentEntry;
  readonly entryNumber: number;
  readonly description: (reference: QuestionRevisionReference) => string;
  readonly bloom: BloomClassificationView | undefined;
}

function bloomFacts(bloom: BloomClassificationView | undefined): JSX.Element | undefined {
  if (bloom === undefined) return undefined;
  return (
    <dl class="assessment-editor-row-facts">
      <div>
        <dt>Bloom Cognitive Process</dt>
        <dd>{bloom.cognitiveProcess}</dd>
      </div>
      <div>
        <dt>Bloom Knowledge Dimension</dt>
        <dd>{bloom.knowledgeDimension}</dd>
      </div>
    </dl>
  );
}

/** Keeps each selected Entry's visible identity and delivery facts easy to scan. */
export function SelectedAssessmentEntryIdentity(
  props: SelectedAssessmentEntryIdentityProps,
): JSX.Element {
  if (props.entry.kind === "fixedQuestion") {
    const question = props.entry;
    return (
      <>
        <h3>
          Entry {props.entryNumber} · {question.reference.questionId} · Revision{" "}
          {question.reference.revisionNumber}
        </h3>
        <p class="assessment-editor-row-description">{props.description(question.reference)}</p>
        {bloomFacts(props.bloom)}
        <dl class="assessment-editor-row-facts">
          <div>
            <dt>Points</dt>
            <dd>{question.pointsPossible}</dd>
          </div>
          <div>
            <dt>Availability</dt>
            <dd>{assessmentEntryAvailabilityLabel(question.availability)}</dd>
          </div>
          <div>
            <dt>Scoring</dt>
            <dd>{assessmentEntryScoringRuleLabel(question.scoringRule)}</dd>
          </div>
          <div>
            <dt>Attempts</dt>
            <dd>{questionAttemptLimitLabel(question)}</dd>
          </div>
          <div>
            <dt>Time limit</dt>
            <dd>{questionTimeLimitLabel(question)}</dd>
          </div>
        </dl>
      </>
    );
  }
  const pool = props.entry;
  return (
    <>
      <h3>
        Entry {props.entryNumber} · Question Pool {pool.questionPoolRevision.questionPoolId} ·
        Revision {pool.questionPoolRevision.revisionNumber}
      </h3>
      <p class="assessment-editor-row-description">
        {pool.selectionRule.selectedQuestionOrder === "randomOrder"
          ? "Random selected Question order"
          : "Question Pool order"}
      </p>
      {bloomFacts(props.bloom)}
      <dl class="assessment-editor-row-facts">
        <div>
          <dt>Questions selected</dt>
          <dd>{pool.selectionCount}</dd>
        </div>
        <div>
          <dt>Points per Question</dt>
          <dd>{pool.pointsPerItem}</dd>
        </div>
        <div>
          <dt>Availability</dt>
          <dd>{assessmentEntryAvailabilityLabel(pool.availability)}</dd>
        </div>
        <div>
          <dt>Scoring</dt>
          <dd>{assessmentEntryScoringRuleLabel(pool.scoringRule)}</dd>
        </div>
        <div>
          <dt>Attempts</dt>
          <dd>{questionAttemptLimitLabel(pool)}</dd>
        </div>
        <div>
          <dt>Time limit</dt>
          <dd>{questionTimeLimitLabel(pool)}</dd>
        </div>
      </dl>
    </>
  );
}
