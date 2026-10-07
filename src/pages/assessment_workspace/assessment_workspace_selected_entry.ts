// Compact, scan-friendly Assessment Entry presentation for the Question editor.

import type { AssessmentEntry } from "../../../generated/api/AssessmentEntry";
import type { PublishedQuestionRevisionTuple } from "../../../generated/api/PublishedQuestionRevisionTuple";
import type { BloomClassificationView } from "../../../generated/api/BloomClassificationView";
import type { RecordContent, RecordFact } from "../../components/record_list/record_list";

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

function labeledFact(label: string, value: string): RecordFact {
  return { kind: "text", label, value };
}

function bloomFacts(bloom: BloomClassificationView | null | undefined): ReadonlyArray<RecordFact> {
  return [
    labeledFact("Bloom Cognitive Process", bloom?.cognitiveProcess ?? "Not assigned"),
    labeledFact("Bloom Knowledge Dimension", bloom?.knowledgeDimension ?? "Not assigned"),
  ];
}

function deliveryFacts(entry: AssessmentEntry): ReadonlyArray<RecordFact> {
  return [
    labeledFact("Availability", assessmentEntryAvailabilityLabel(entry.availability)),
    labeledFact("Scoring", assessmentEntryScoringRuleLabel(entry.scoringRule)),
    labeledFact("Attempts", questionAttemptLimitLabel(entry)),
    labeledFact("Time limit", questionTimeLimitLabel(entry)),
  ];
}

/** Exact Question or Pool identity, with delivery facts the Question editor still scans. */
export function selectedAssessmentEntryContent(args: {
  readonly entry: AssessmentEntry;
  readonly entryNumber: number;
  readonly questionTitle: (
    publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple,
  ) => string;
  readonly poolTitle: (questionPoolId: string) => string;
  readonly description: (publishedQuestionRevisionTuple: PublishedQuestionRevisionTuple) => string;
  readonly bloom: BloomClassificationView | undefined;
  readonly removeDisabled: boolean;
  readonly remove: () => void;
}): RecordContent {
  const entry = args.entry;
  const description =
    entry.kind === "fixedQuestion"
      ? args.description(entry.publishedQuestionRevisionTuple)
      : "Random selection from Question Pool";
  const identity =
    entry.kind === "fixedQuestion"
      ? [
          labeledFact("Question ID", entry.publishedQuestionRevisionTuple.publishedQuestionId),
          labeledFact("Revision", String(entry.publishedQuestionRevisionTuple.revisionNumber)),
          labeledFact("Points", entry.pointsPossible),
        ]
      : [
          labeledFact("Question Pool ID", entry.questionPoolId),
          labeledFact("Edit", String(entry.questionPoolEditNumber)),
          labeledFact("Questions selected", String(entry.selectionCount)),
          labeledFact("Points per Question", entry.pointsPerItem),
        ];
  return {
    title:
      entry.kind === "fixedQuestion"
        ? args.questionTitle(entry.publishedQuestionRevisionTuple)
        : args.poolTitle(entry.questionPoolId),
    description,
    details: [
      labeledFact("Entry", String(args.entryNumber)),
      ...identity,
      ...bloomFacts(args.bloom),
      ...deliveryFacts(entry),
    ],
    actions: [
      {
        id: "remove",
        kind: "command",
        label: entry.availability === "available" ? "Remove" : "Retained unavailable",
        disabled: args.removeDisabled || entry.availability !== "available",
        title:
          entry.availability === "available"
            ? `Remove Assessment Entry ${args.entryNumber}`
            : `Retained unavailable Assessment Entry ${args.entryNumber} cannot be removed`,
        onClick: args.remove,
      },
    ],
  };
}
