// Visible shared, added, removed, and changed Blueprint comparison membership.

import type { JSX } from "solid-js";
import type { BlueprintComparisonView } from "../../../generated/api/BlueprintComparisonView";
import {
  assessmentComparison,
  publishedQuestionComparison,
  questionPoolComparison,
  type ComparisonMembership,
} from "./blueprint_fork_model";

function membershipLine(label: string, membership: readonly string[]): JSX.Element {
  return (
    <p>
      {label}: {membership.length > 0 ? membership.join(", ") : "None"}.
    </p>
  );
}

function membershipBlock(title: string, membership: ComparisonMembership): JSX.Element {
  return (
    <>
      {membershipLine(`Shared ${title}`, membership.shared)}
      {membershipLine(`Added ${title}`, membership.added)}
      {membershipLine(`Removed ${title}`, membership.removed)}
      {membershipLine(`Changed ${title}`, membership.changed)}
    </>
  );
}

/** Reads the current comparison inventories and states each shared, added, removed, and changed row. */
export function ComparisonMembershipSummary(props: {
  readonly view: BlueprintComparisonView;
}): JSX.Element {
  return (
    <section data-blueprint-comparison-membership>
      <h3>Shared, added, removed, and changed</h3>
      {membershipBlock("Assessments", assessmentComparison(props.view))}
      {membershipBlock("Published Questions", publishedQuestionComparison(props.view))}
      {membershipBlock("Question Pools", questionPoolComparison(props.view))}
    </section>
  );
}
