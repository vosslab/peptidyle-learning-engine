// instructor_setup_checkpoint.ts - private, redacted progress marker for a failed instructor child.

import {
  closeSync,
  constants,
  fchmodSync,
  fsyncSync,
  fstatSync,
  ftruncateSync,
  lstatSync,
  openSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname } from "node:path";

const CHECKPOINT_FILE = "instructor-setup-checkpoint.txt";

export const INSTRUCTOR_SETUP_CHECKPOINTS = [
  "browser_ready",
  "login_visible",
  "signed_in",
  "course_created",
  "course_opened",
  "student_active",
  "assignment_editor_opened",
  "catalog_result_selected",
  "assignment_created",
] as const;

export type InstructorSetupCheckpoint = (typeof INSTRUCTOR_SETUP_CHECKPOINTS)[number];

function isCheckpoint(value: string): value is InstructorSetupCheckpoint {
  return (INSTRUCTOR_SETUP_CHECKPOINTS as readonly string[]).includes(value);
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

/** Updates the runner-owned inode with only the last completed visible instructor-setup stage. */
export function writeInstructorSetupCheckpoint(
  path: string,
  checkpoint: InstructorSetupCheckpoint,
  afterWrite: (() => void) | undefined = undefined,
): void {
  if (!isCheckpoint(checkpoint)) throw new Error("instructor setup checkpoint is invalid");
  const parentPath = dirname(path);
  if (basename(path) !== CHECKPOINT_FILE || parentPath === ".") {
    throw new Error("instructor setup checkpoint path is unsafe");
  }
  const directory = openSync(
    parentPath,
    constants.O_RDONLY | (constants.O_DIRECTORY ?? 0) | (constants.O_NOFOLLOW ?? 0),
  );
  let descriptor = -1;
  try {
    const parent = fstatSync(directory);
    if (
      !parent.isDirectory() ||
      (parent.mode & 0o777) !== 0o700 ||
      !namedParentMatches(parentPath, parent)
    ) {
      throw new Error("instructor setup checkpoint path is unsafe");
    }
    try {
      descriptor = openSync(path, constants.O_WRONLY | (constants.O_NOFOLLOW ?? 0));
      const metadata = fstatSync(descriptor);
      const namedTarget = lstatSync(path);
      if (
        !metadata.isFile() ||
        (metadata.mode & 0o777) !== 0o600 ||
        !namedTarget.isFile() ||
        namedTarget.isSymbolicLink() ||
        namedTarget.dev !== metadata.dev ||
        namedTarget.ino !== metadata.ino
      ) {
        throw new Error("instructor setup checkpoint path is unsafe");
      }
    } catch (error: unknown) {
      if (
        error instanceof Error &&
        error.message === "instructor setup checkpoint path is unsafe"
      ) {
        throw error;
      }
      throw Object.assign(new Error("instructor setup checkpoint path is unsafe"), {
        cause: error,
      });
    }
    ftruncateSync(descriptor, 0);
    writeFileSync(descriptor, `${checkpoint}\n`, "ascii");
    fchmodSync(descriptor, 0o600);
    fsyncSync(descriptor);
    fsyncSync(directory);
    const committed = fstatSync(descriptor);
    afterWrite?.();
    const namedTarget = lstatSync(path);
    if (
      !namedParentMatches(parentPath, parent) ||
      !namedTarget.isFile() ||
      namedTarget.isSymbolicLink() ||
      (namedTarget.mode & 0o777) !== 0o600 ||
      namedTarget.dev !== committed.dev ||
      namedTarget.ino !== committed.ino
    ) {
      throw new Error("instructor setup checkpoint path is unsafe");
    }
  } finally {
    if (descriptor >= 0) closeSync(descriptor);
    closeSync(directory);
  }
}
