// j1_checkpoint.spec.ts - private J1 failure checkpoint boundary tests.

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

import { writeJ1Checkpoint } from "./j1_checkpoint";
import { writeJ2Checkpoint } from "./j2_checkpoint";

const temporaryDirectories: string[] = [];

test.afterEach(() => {
  for (const directory of temporaryDirectories.splice(0)) {
    rmSync(directory, { recursive: true, force: true });
  }
});

function checkpointPath(filename = "j1-checkpoint.txt"): string {
  const directory = mkdtempSync(join(tmpdir(), "ple-j1-checkpoint-"));
  temporaryDirectories.push(directory);
  chmodSync(directory, 0o700);
  const path = join(directory, filename);
  writeFileSync(path, "", { encoding: "ascii", mode: 0o600 });
  return path;
}

test("journey checkpoint atomically records a closed, ASCII visible stage", () => {
  const path = checkpointPath();
  writeJ1Checkpoint(path, "course_opened");
  expect(readFileSync(path, "ascii")).toBe("course_opened\n");
  expect(lstatSync(path).isFile()).toBe(true);
  expect(lstatSync(path).mode & 0o777).toBe(0o600);
});

test("J2 checkpoint records the last safe first-run milestone", () => {
  const path = checkpointPath("j2-checkpoint.txt");
  writeJ2Checkpoint(path, "first_run_completed");
  expect(readFileSync(path, "ascii")).toBe("first_run_completed\n");
});

test("J1 checkpoint rejects unknown stages and hostile private filesystem shapes", () => {
  const path = checkpointPath();
  expect(() => writeJ1Checkpoint(path, "answer_visible" as never)).toThrow("invalid");
  chmodSync(path, 0o644);
  expect(() => writeJ1Checkpoint(path, "signed_in")).toThrow("unsafe");
  chmodSync(path, 0o600);
  const hostileDirectory = mkdtempSync(join(tmpdir(), "ple-j1-checkpoint-hostile-"));
  temporaryDirectories.push(hostileDirectory);
  chmodSync(hostileDirectory, 0o700);
  const outside = join(hostileDirectory, "outside.txt");
  writeFileSync(outside, "", { encoding: "ascii", mode: 0o600 });
  const linkPath = join(hostileDirectory, "j1-checkpoint.txt");
  symlinkSync(outside, linkPath);
  expect(() => writeJ1Checkpoint(linkPath, "signed_in")).toThrow("unsafe");
});

test("J1 checkpoint fails closed when its named parent changes after atomic rename", () => {
  const path = checkpointPath();
  const parent = dirname(path);
  const replacementRoot = mkdtempSync(join(tmpdir(), "ple-j1-checkpoint-replacement-"));
  temporaryDirectories.push(replacementRoot);
  chmodSync(replacementRoot, 0o700);
  expect(() =>
    writeJ1Checkpoint(path, "signed_in", () => {
      renameSync(parent, join(replacementRoot, "moved"));
      mkdirSync(parent, { mode: 0o700 });
    }),
  ).toThrow();
});
