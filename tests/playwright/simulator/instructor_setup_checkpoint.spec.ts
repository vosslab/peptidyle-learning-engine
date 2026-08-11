// instructor_setup_checkpoint.spec.ts - private instructor checkpoint boundary tests.

import {
  chmodSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  renameSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";

import { expect, test } from "@playwright/test";

import { writeInstructorSetupCheckpoint } from "./instructor_setup_checkpoint";

const temporaryDirectories: string[] = [];

test.afterEach(() => {
  for (const directory of temporaryDirectories.splice(0)) {
    rmSync(directory, { recursive: true, force: true });
  }
});

function checkpointPath(): string {
  const directory = mkdtempSync(join(tmpdir(), "ple-instructor-checkpoint-"));
  temporaryDirectories.push(directory);
  chmodSync(directory, 0o700);
  const path = join(directory, "instructor-setup-checkpoint.txt");
  writeFileSync(path, "", { encoding: "ascii", mode: 0o600 });
  return path;
}

test("instructor setup checkpoint records one closed ASCII visible stage in the runner-owned inode", () => {
  const path = checkpointPath();
  writeInstructorSetupCheckpoint(path, "course_opened");
  expect(readFileSync(path, "ascii")).toBe("course_opened\n");
  expect(lstatSync(path).isFile()).toBe(true);
  expect(lstatSync(path).mode & 0o777).toBe(0o600);
});

test("instructor setup checkpoint rejects unknown stages and hostile private filesystem shapes", () => {
  const path = checkpointPath();
  expect(() => writeInstructorSetupCheckpoint(path, "answer_visible" as never)).toThrow("invalid");
  chmodSync(path, 0o644);
  expect(() => writeInstructorSetupCheckpoint(path, "signed_in")).toThrow("unsafe");
  chmodSync(path, 0o600);
  const hostileDirectory = mkdtempSync(join(tmpdir(), "ple-instructor-checkpoint-hostile-"));
  temporaryDirectories.push(hostileDirectory);
  chmodSync(hostileDirectory, 0o700);
  const outside = join(hostileDirectory, "outside.txt");
  writeFileSync(outside, "", { encoding: "ascii", mode: 0o600 });
  const linkPath = join(hostileDirectory, "instructor-setup-checkpoint.txt");
  symlinkSync(outside, linkPath);
  expect(() => writeInstructorSetupCheckpoint(linkPath, "signed_in")).toThrow("unsafe");
});

test("instructor setup checkpoint fails closed when its named parent changes after the write", () => {
  const path = checkpointPath();
  const parent = dirname(path);
  const replacementRoot = mkdtempSync(join(tmpdir(), "ple-instructor-checkpoint-replacement-"));
  temporaryDirectories.push(replacementRoot);
  chmodSync(replacementRoot, 0o700);
  expect(() =>
    writeInstructorSetupCheckpoint(path, "signed_in", () => {
      renameSync(parent, join(replacementRoot, "moved"));
      mkdirSync(parent, { mode: 0o700 });
    }),
  ).toThrow();
});

test("instructor setup checkpoint rejects a same-parent regular-file replacement after the write", () => {
  const path = checkpointPath();
  expect(() =>
    writeInstructorSetupCheckpoint(path, "signed_in", () => {
      const replacement = `${path}.replacement`;
      writeFileSync(replacement, "assignment_created\n", { encoding: "ascii", mode: 0o600 });
      renameSync(replacement, path);
    }),
  ).toThrow("unsafe");
});
