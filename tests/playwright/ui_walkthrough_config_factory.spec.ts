// ui_walkthrough_config_factory.spec.ts - offline behavior for explicit private walkthrough input.

import { chmodSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { expect, test } from "@playwright/test";

import {
  createUiWalkthroughConfig,
  readUiWalkthroughInputs,
} from "./ui_walkthrough_config_factory";

const CREDENTIALS = [
  "instructor=fixture_instructor_credential_that_is_long_enough_for_local_provider",
  "student=fixture_student_credential_that_is_long_enough_for_the_local_provider",
].join("\n");

function privateInputFile(value: Record<string, unknown>): {
  readonly directory: string;
  readonly input: string;
} {
  const directory = mkdtempSync(join(tmpdir(), "ple-config-input-"));
  chmodSync(directory, 0o700);
  const credentialFile = join(directory, "credentials.txt");
  const journeyStateFile = join(directory, "journey-state.json");
  const input = join(directory, "walkthrough-inputs.json");
  writeFileSync(credentialFile, CREDENTIALS, { encoding: "ascii", mode: 0o600 });
  writeFileSync(journeyStateFile, "{}", { encoding: "ascii", mode: 0o600 });
  writeFileSync(
    input,
    JSON.stringify({
      schemaVersion: 1,
      stage: "learner_journey",
      baseUrl: "http://127.0.0.1:3000",
      masterSeed: 42,
      credentialFile,
      journeyStateFile,
      j1CheckpointFile: join(directory, "j1-checkpoint.txt"),
      j2CheckpointFile: join(directory, "j2-checkpoint.txt"),
      courseReference: "C-42",
      masteryAssignmentReference: "A-73",
      screenshotDirectory: null,
      ...value,
    }),
    { encoding: "ascii", mode: 0o600 },
  );
  return { directory, input };
}

function privateInstructorInputFile(catalogDisplayIds: unknown): {
  readonly directory: string;
  readonly input: string;
} {
  const directory = mkdtempSync(join(tmpdir(), "ple-instructor-config-input-"));
  chmodSync(directory, 0o700);
  const credentialFile = join(directory, "credentials.txt");
  const journeyStateFile = join(directory, "journeys.json");
  const input = join(directory, "walkthrough-inputs.json");
  writeFileSync(credentialFile, CREDENTIALS, { encoding: "ascii", mode: 0o600 });
  writeFileSync(journeyStateFile, "", { encoding: "ascii", mode: 0o600 });
  writeFileSync(
    input,
    JSON.stringify({
      schemaVersion: 1,
      stage: "instructor_setup",
      baseUrl: "http://127.0.0.1:3000",
      masterSeed: 42,
      credentialFile,
      journeyStateFile,
      instructorSetupCheckpointFile: join(directory, "instructor-setup-checkpoint.txt"),
      catalogDisplayIds,
      screenshotDirectory: null,
    }),
    { encoding: "ascii", mode: 0o600 },
  );
  return { directory, input };
}

test("explicit learner input configures the live origin without environment state", () => {
  const temporary = privateInputFile({});
  try {
    const inputs = readUiWalkthroughInputs(temporary.input);
    const testDirectory = join(tmpdir(), "ple-playwright-tests");
    const config = createUiWalkthroughConfig(temporary.input, testDirectory);
    expect(inputs).toMatchObject({ stage: "learner_journey", masterSeed: 42 });
    expect(config.use).toMatchObject({ baseURL: "http://127.0.0.1:3000" });
    expect(config.testDir).toBe(testDirectory);
  } finally {
    rmSync(temporary.directory, { recursive: true, force: true });
  }
});

test("learner input rejects an internal UUID in place of a visible route reference", () => {
  const temporary = privateInputFile({ courseReference: "123e4567-e89b-12d3-a456-426614174000" });
  try {
    expect(() => readUiWalkthroughInputs(temporary.input)).toThrow("course reference is invalid");
  } finally {
    rmSync(temporary.directory, { recursive: true, force: true });
  }
});

test("explicit input rejects an unrecognized stage before browser creation", () => {
  const temporary = privateInputFile({ stage: "arrange" });
  try {
    expect(() => readUiWalkthroughInputs(temporary.input)).toThrow("stage is invalid");
  } finally {
    rmSync(temporary.directory, { recursive: true, force: true });
  }
});

test("explicit input rejects a noncanonical private JSON file", () => {
  const temporary = privateInputFile({});
  try {
    const text = '{ "schemaVersion": 1 }';
    writeFileSync(temporary.input, text, { encoding: "ascii", mode: 0o600 });
    expect(() => readUiWalkthroughInputs(temporary.input)).toThrow("canonical JSON");
  } finally {
    rmSync(temporary.directory, { recursive: true, force: true });
  }
});

test("instructor input retains only four human-readable catalog IDs", () => {
  const temporary = privateInstructorInputFile(["7K3-M9QP", "ABC-123T", "PEP-T1D3", "GEN-E42K"]);
  try {
    expect(readUiWalkthroughInputs(temporary.input)).toMatchObject({
      stage: "instructor_setup",
      catalogDisplayIds: ["7K3-M9QP", "ABC-123T", "PEP-T1D3", "GEN-E42K"],
    });
    writeFileSync(
      temporary.input,
      JSON.stringify({
        schemaVersion: 1,
        stage: "instructor_setup",
        baseUrl: "http://127.0.0.1:3000",
        masterSeed: 42,
        credentialFile: join(temporary.directory, "credentials.txt"),
        journeyStateFile: join(temporary.directory, "journeys.json"),
        instructorSetupCheckpointFile: join(temporary.directory, "instructor-setup-checkpoint.txt"),
        catalogDisplayIds: [
          {
            displayId: "7K3-M9QP",
            problemId: "A-74",
            versionId: "123e4567-e89b-12d3-a456-426614174003",
          },
          "ABC-123T",
          "PEP-T1D3",
          "GEN-E42K",
        ],
        screenshotDirectory: null,
      }),
      { encoding: "ascii", mode: 0o600 },
    );
    expect(() => readUiWalkthroughInputs(temporary.input)).toThrow("Question ID is invalid");
  } finally {
    rmSync(temporary.directory, { recursive: true, force: true });
  }
});
