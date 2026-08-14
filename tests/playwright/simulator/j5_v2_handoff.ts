// j5_v2_handoff.ts - owned public-ID seam until WP-E1 joins the schema-v2 state contract.

import { isAssignmentReference, isCourseReference } from "./public_references";

export interface J5V2Input {
  readonly courseReference: string;
  readonly assignmentReference: string;
}

/**
 * Validates only the public route references that WP-E1 will supply from J13.
 * Assignment title, learner identity, scores, and run details stay browser-only.
 */
export function j5V2Input(courseReference: string, assignmentReference: string): J5V2Input {
  if (!isCourseReference(courseReference) || !isAssignmentReference(assignmentReference)) {
    throw new Error("J5 requires canonical public course and assignment references");
  }
  return { courseReference, assignmentReference };
}
