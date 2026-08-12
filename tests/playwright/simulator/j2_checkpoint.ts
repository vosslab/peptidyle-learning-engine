// j2_checkpoint.ts - safe stage vocabulary for the second learner journey child.

import { writeJourneyCheckpoint, type JourneyCheckpointDefinition } from "./journey_checkpoint";

export const J2_CHECKPOINTS = [
  "signed_in",
  "active_run_visible",
  "response_selected",
  "feedback_visible",
  "first_run_completed",
  "fresh_practice_visible",
] as const;

export type J2Checkpoint = (typeof J2_CHECKPOINTS)[number];

const J2_CHECKPOINT_DEFINITION: JourneyCheckpointDefinition<J2Checkpoint> = {
  fileName: "j2-checkpoint.txt",
  stages: J2_CHECKPOINTS,
};

/** Atomically records only the last completed safe, visible J2 stage. */
export function writeJ2Checkpoint(
  path: string,
  checkpoint: J2Checkpoint,
  afterRename: (() => void) | undefined = undefined,
): void {
  writeJourneyCheckpoint(J2_CHECKPOINT_DEFINITION, path, checkpoint, afterRename);
}
