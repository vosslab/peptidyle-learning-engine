// journey_state.ts - fail-closed private J1/J2 handoff outside browser artifacts.

import {
  closeSync,
  constants,
  fchmodSync,
  fstatSync,
  fsyncSync,
  ftruncateSync,
  lstatSync,
  openSync,
  readSync,
  writeFileSync,
} from "node:fs";

import { parseJourneyPrefix, type JourneyFragment } from "./visible_outcome_report";

const MAX_STATE_BYTES = 4096;

/** Reads only an exact canonical private prefix through a no-follow descriptor. */
export function readJourneyStatePrefix(path: string): readonly JourneyFragment[] {
  const parent = lstatSync(parentPath(path));
  const namedFile = lstatSync(path);
  if (
    !parent.isDirectory() ||
    parent.isSymbolicLink() ||
    (parent.mode & 0o777) !== 0o700 ||
    !namedFile.isFile() ||
    namedFile.isSymbolicLink() ||
    (namedFile.mode & 0o777) !== 0o600
  )
    throw new Error("private journey state is unsafe");
  const descriptor = openSync(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  try {
    const metadata = fstatSync(descriptor);
    if (!metadata.isFile() || (metadata.mode & 0o777) !== 0o600 || metadata.size > MAX_STATE_BYTES)
      throw new Error("private journey state is unsafe");
    return parsePrefix(readAscii(descriptor, metadata.size));
  } finally {
    closeSync(descriptor);
  }
}

/** Appends the next fixed public journey only to the exact private canonical prefix. */
export function appendJourneyState(path: string, journey: JourneyFragment): void {
  const parent = lstatSync(parentPath(path));
  if (!parent.isDirectory() || parent.isSymbolicLink() || (parent.mode & 0o777) !== 0o700) {
    throw new Error("private journey state parent is unsafe");
  }
  const namedFile = lstatSync(path);
  if (!namedFile.isFile() || namedFile.isSymbolicLink() || (namedFile.mode & 0o777) !== 0o600) {
    throw new Error("private journey state is unsafe");
  }
  const noFollow = constants.O_NOFOLLOW ?? 0;
  const descriptor = openSync(path, constants.O_RDWR | noFollow);
  try {
    const metadata = fstatSync(descriptor);
    if (
      !metadata.isFile() ||
      (metadata.mode & 0o777) !== 0o600 ||
      metadata.size > MAX_STATE_BYTES
    ) {
      throw new Error("private journey state is unsafe");
    }
    const raw = readAscii(descriptor, metadata.size);
    const prefix = parsePrefix(raw);
    const expectedJourney = ["J1", "J2", "J3", "J4", "J5", "J8"][prefix.length];
    if (expectedJourney !== journey.journey || !matchesPrefix(prefix, journey))
      throw new Error("private journey state does not match next journey");
    const output = JSON.stringify([...prefix, journey]) + "\n";
    ftruncateSync(descriptor, 0);
    writeFileSync(descriptor, output, "ascii");
    fchmodSync(descriptor, 0o600);
    fsyncSync(descriptor);
  } finally {
    closeSync(descriptor);
  }
}

/** Compatibility wrapper for J2's existing accepted public fragment. */
export function appendW2JourneyState(path: string, journey: JourneyFragment): void {
  appendJourneyState(path, journey);
}

function parentPath(path: string): string {
  const separator = path.lastIndexOf("/");
  if (separator <= 0) throw new Error("private journey state path is unsafe");
  return path.slice(0, separator);
}

function readAscii(descriptor: number, size: number): string {
  const bytes = Buffer.alloc(size);
  const read = readSync(descriptor, bytes, 0, size, 0);
  if (read !== size || bytes.some((byte) => byte > 0x7f)) {
    throw new Error("private journey state is unsafe");
  }
  return bytes.toString("ascii");
}

function parsePrefix(raw: string): readonly JourneyFragment[] {
  if (
    raw.length === 0 ||
    !raw.endsWith("\n") ||
    raw.includes("\r") ||
    raw.split("\n").length !== 2
  ) {
    throw new Error("private journey state is unsafe");
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(raw);
  } catch {
    throw new Error("private journey state is unsafe");
  }
  if (raw !== JSON.stringify(parsed) + "\n") {
    throw new Error("private journey state is unsafe");
  }
  const prefix = parseJourneyPrefix(parsed);
  if (prefix === undefined) throw new Error("private journey state is unsafe");
  return prefix;
}

function matchesPrefix(prefix: readonly JourneyFragment[], next: JourneyFragment): boolean {
  if (prefix.length === 0) return next.journey === "J1";
  const first = prefix[0];
  if (first === undefined || first.journey !== "J1") return false;
  if (next.journey === "J4")
    return (
      next.courseReference === first.courseReference &&
      next.masteryAssignmentReference === first.assignmentReference
    );
  return (
    next.courseReference === first.courseReference &&
    next.assignmentReference === first.assignmentReference
  );
}
