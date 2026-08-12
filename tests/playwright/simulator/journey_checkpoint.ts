// journey_checkpoint.ts - private, redacted progress marker for a failed journey child.

import { randomUUID } from "node:crypto";
import {
  closeSync,
  constants,
  fchmodSync,
  fsyncSync,
  fstatSync,
  lstatSync,
  openSync,
  renameSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join } from "node:path";

export interface JourneyCheckpointDefinition<Checkpoint extends string> {
  readonly fileName: string;
  readonly stages: readonly Checkpoint[];
}

function namedParentMatches(parentPath: string, parent: ReturnType<typeof fstatSync>): boolean {
  const namedParent = lstatSync(parentPath);
  return (
    namedParent.isDirectory() &&
    !namedParent.isSymbolicLink() &&
    (namedParent.mode & 0o777) === 0o700 &&
    namedParent.dev === parent.dev &&
    namedParent.ino === parent.ino
  );
}

/** Atomically records only the last completed safe, visible journey stage. */
export function writeJourneyCheckpoint<Checkpoint extends string>(
  definition: JourneyCheckpointDefinition<Checkpoint>,
  path: string,
  checkpoint: Checkpoint,
  afterRename: (() => void) | undefined = undefined,
): void {
  if (!definition.stages.includes(checkpoint)) throw new Error("journey checkpoint is invalid");
  const parentPath = dirname(path);
  if (basename(path) !== definition.fileName || parentPath === ".") {
    throw new Error("journey checkpoint path is unsafe");
  }
  const directory = openSync(
    parentPath,
    constants.O_RDONLY | (constants.O_DIRECTORY ?? 0) | (constants.O_NOFOLLOW ?? 0),
  );
  let descriptor = -1;
  let temporaryPath: string | undefined;
  try {
    const parent = fstatSync(directory);
    if (
      !parent.isDirectory() ||
      (parent.mode & 0o777) !== 0o700 ||
      !namedParentMatches(parentPath, parent)
    ) {
      throw new Error("journey checkpoint path is unsafe");
    }
    let existing = -1;
    try {
      existing = openSync(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
      const metadata = fstatSync(existing);
      if (!metadata.isFile() || (metadata.mode & 0o777) !== 0o600) {
        throw new Error("journey checkpoint path is unsafe");
      }
    } catch (error: unknown) {
      if (error instanceof Error && error.message === "journey checkpoint path is unsafe") {
        throw error;
      }
      throw Object.assign(new Error("journey checkpoint path is unsafe"), { cause: error });
    } finally {
      if (existing >= 0) closeSync(existing);
    }
    temporaryPath = join(parentPath, `.${definition.fileName}.${randomUUID()}`);
    descriptor = openSync(
      temporaryPath,
      constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | (constants.O_NOFOLLOW ?? 0),
      0o600,
    );
    writeFileSync(descriptor, `${checkpoint}\n`, "ascii");
    fchmodSync(descriptor, 0o600);
    fsyncSync(descriptor);
    closeSync(descriptor);
    descriptor = -1;
    renameSync(temporaryPath, path);
    temporaryPath = undefined;
    fsyncSync(directory);
    afterRename?.();
    const committed = lstatSync(path);
    if (
      !namedParentMatches(parentPath, parent) ||
      !committed.isFile() ||
      committed.isSymbolicLink() ||
      (committed.mode & 0o777) !== 0o600
    ) {
      throw new Error("journey checkpoint path is unsafe");
    }
  } finally {
    if (descriptor >= 0) closeSync(descriptor);
    if (temporaryPath !== undefined) {
      try {
        unlinkSync(temporaryPath);
      } catch {
        // The diagnostic never changes learner-facing behavior.
      }
    }
    closeSync(directory);
  }
}
