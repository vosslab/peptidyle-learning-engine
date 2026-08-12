// Explicit private-input boundary tests for the live walkthrough Playwright configuration.

import { chmodSync, mkdtempSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { expect, test } from "@playwright/test";

import {
  createUiWalkthroughConfig,
  readUiWalkthroughInputs,
} from "./ui_walkthrough_config_factory";

function writePrivateLearnerInputs(directory: string): string {
  const credentialFile = join(directory, "local-login.txt");
  const journeyStateFile = join(directory, "journeys.json");
  const inputPath = join(directory, "walkthrough-inputs.json");
  writeFileSync(credentialFile, "student=student_credential_which_is_long_enough_for_local_use\n", {
    encoding: "ascii",
    mode: 0o600,
  });
  writeFileSync(journeyStateFile, "{}", { encoding: "ascii", mode: 0o600 });
  writeFileSync(
    inputPath,
    JSON.stringify({
      schemaVersion: 1,
      stage: "learner_journey",
      baseUrl: "http://127.0.0.1:3010",
      masterSeed: 42,
      credentialFile,
      journeyStateFile,
      j1CheckpointFile: join(directory, "j1-checkpoint.txt"),
      j2CheckpointFile: join(directory, "j2-checkpoint.txt"),
      courseId: "123e4567-e89b-12d3-a456-426614174000",
      masteryAssignmentId: "123e4567-e89b-12d3-a456-426614174001",
      screenshotDirectory: null,
    }),
    { encoding: "ascii", mode: 0o600 },
  );
  return inputPath;
}

test("explicit private inputs select the browser origin despite inherited PLE state", () => {
  const directory = mkdtempSync(join(tmpdir(), "ple-live-config-"));
  chmodSync(directory, 0o700);
  const prior = process.env["PLE_GATEWAY_HOST_PORT"];
  process.env["PLE_GATEWAY_HOST_PORT"] = "3999";
  try {
    const inputPath = writePrivateLearnerInputs(directory);
    const testDirectory = join(directory, "playwright-tests");
    const config = createUiWalkthroughConfig(inputPath, testDirectory);
    if (config.use === undefined)
      throw new Error("walkthrough config must define browser use data");
    expect(config.use.baseURL).toBe("http://127.0.0.1:3010");
    expect(config.testDir).toBe(testDirectory);
  } finally {
    if (prior === undefined) delete process.env["PLE_GATEWAY_HOST_PORT"];
    else process.env["PLE_GATEWAY_HOST_PORT"] = prior;
    rmSync(directory, { recursive: true, force: true });
  }
});

test("unsafe private input metadata is rejected before browser configuration", () => {
  const directory = mkdtempSync(join(tmpdir(), "ple-live-config-"));
  chmodSync(directory, 0o700);
  try {
    const inputPath = writePrivateLearnerInputs(directory);
    chmodSync(inputPath, 0o644);
    expect(() => readUiWalkthroughInputs(inputPath)).toThrow("unsafe metadata");
    chmodSync(inputPath, 0o600);
    const linkPath = join(directory, "linked-inputs.json");
    symlinkSync(inputPath, linkPath);
    expect(() => readUiWalkthroughInputs(linkPath)).toThrow("unsafe metadata");
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
