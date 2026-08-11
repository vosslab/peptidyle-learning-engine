// j5_v2_handoff.ts - owned public-ID seam until WP-E1 joins the schema-v2 state contract.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

export interface J5V2Input {
  readonly courseId: string;
  readonly assignmentId: string;
}

/**
 * Validates only the public identifiers that WP-E1 will supply from J13.
 * Assignment title, learner identity, scores, and run details stay browser-only.
 */
export function j5V2Input(courseId: string, assignmentId: string): J5V2Input {
  if (!UUID.test(courseId) || !UUID.test(assignmentId)) {
    throw new Error("J5 requires canonical public course and assignment identifiers");
  }
  return { courseId, assignmentId };
}
