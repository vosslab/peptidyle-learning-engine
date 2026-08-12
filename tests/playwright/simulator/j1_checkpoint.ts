// j1_checkpoint.ts - safe stage vocabulary for the first learner journey child.

import { writeJourneyCheckpoint, type JourneyCheckpointDefinition } from "./journey_checkpoint";

export const J1_CHECKPOINTS = [
  "signed_in",
  "course_visible",
  "course_opened",
  "assignment_visible",
  "run_controls_visible",
  "feedback_visible",
  "next_question_visible",
] as const;

export type J1Checkpoint = (typeof J1_CHECKPOINTS)[number];

const J1_CHECKPOINT_DEFINITION: JourneyCheckpointDefinition<J1Checkpoint> = {
  fileName: "j1-checkpoint.txt",
  stages: J1_CHECKPOINTS,
};

/** Atomically records only the last completed safe, visible J1 stage. */
export function writeJ1Checkpoint(
  path: string,
  checkpoint: J1Checkpoint,
  afterRename: (() => void) | undefined = undefined,
): void {
  writeJourneyCheckpoint(J1_CHECKPOINT_DEFINITION, path, checkpoint, afterRename);
}
