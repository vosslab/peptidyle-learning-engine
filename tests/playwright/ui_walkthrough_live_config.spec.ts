// ui_walkthrough_live_config.spec.ts - offline validation for gateway-smoke inputs.

import { expect, test } from "@playwright/test";

import { mockPreviewServerEnabled, outputDirectoryForUiWalkthrough } from "../../playwright.config";
import { liveModeActivationFromEnvironment } from "./live_mode_activation";
import {
  instructorSetupInputsFromEnvironment,
  instructorCredentialFromValidatedFile,
  learnerAliasFromValidatedFile,
  studentCredentialFromValidatedFile,
  uiWalkthroughInputsFromEnvironment,
} from "./ui_walkthrough_live_config";

const CREDENTIAL = "student=fixture_credential_that_is_long_enough_for_the_local_provider";
const INSTRUCTOR_CREDENTIAL =
  "instructor=fixture_instructor_credential_that_is_long_enough_for_local_provider";
const CREDENTIAL_FILE = `${INSTRUCTOR_CREDENTIAL}
${CREDENTIAL}`;
const COMPLETE_ENVIRONMENT = {
  PLE_UI_WALKTHROUGH_LIVE_REQUIRED: "1",
  PLE_UI_WALKTHROUGH_LIVE_BASE_URL: "http://127.0.0.1:3000",
  PLE_UI_WALKTHROUGH_LIVE_CREDENTIAL_FILE: "fixture-local-login.txt",
  PLE_UI_WALKTHROUGH_MASTER_SEED: "00042",
  PLE_UI_WALKTHROUGH_LIVE_COURSE_ID: "123e4567-e89b-12d3-a456-426614174000",
  PLE_UI_WALKTHROUGH_LIVE_MASTERY_ASSIGNMENT_ID: "123e4567-e89b-12d3-a456-426614174001",
  PLE_UI_WALKTHROUGH_LIVE_MASTERY_PROBLEM_ID: "123e4567-e89b-12d3-a456-426614174003",
  PLE_UI_WALKTHROUGH_LIVE_EXAM_ASSIGNMENT_ID: "123e4567-e89b-12d3-a456-426614174002",
  PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE: "/private/tmp/fixture-journey-state.json",
};

test("walkthrough activation validates live inputs before browser creation", () => {
  expect(uiWalkthroughInputsFromEnvironment({}, () => undefined)).toBeUndefined();
  expect(
    uiWalkthroughInputsFromEnvironment({ PLE_UI_WALKTHROUGH_LIVE_REQUIRED: "0" }, () => undefined),
  ).toBeUndefined();
  expect(() =>
    uiWalkthroughInputsFromEnvironment(
      { PLE_UI_WALKTHROUGH_LIVE_REQUIRED: "yes" },
      () => undefined,
    ),
  ).toThrow("exactly 1");
  expect(() =>
    uiWalkthroughInputsFromEnvironment({ PLE_UI_WALKTHROUGH_LIVE_REQUIRED: "1" }),
  ).toThrow("PLE_UI_WALKTHROUGH_LIVE_BASE_URL");
  expect(
    uiWalkthroughInputsFromEnvironment(
      COMPLETE_ENVIRONMENT,
      () => undefined,
      () => undefined,
    ),
  ).toMatchObject({
    baseUrl: "http://127.0.0.1:3000",
    credentialFile: "fixture-local-login.txt",
    masterSeed: 42,
    masterSeedText: "42",
    journeyStateFile: "/private/tmp/fixture-journey-state.json",
    journeyArtifactsDirectory: "/private/tmp/journey-artifacts",
  });
});

test("walkthrough base URL rejects unsafe variants and non-loopback http", () => {
  for (const baseUrl of [
    "ftp://127.0.0.1:3000",
    "http://user:password@127.0.0.1:3000",
    "http://127.0.0.1:3000/path",
    "http://127.0.0.1:3000/?query=value",
    "http://127.0.0.1:3000/#fragment",
    "http://example.test:3000",
  ]) {
    expect(() =>
      uiWalkthroughInputsFromEnvironment(
        { ...COMPLETE_ENVIRONMENT, PLE_UI_WALKTHROUGH_LIVE_BASE_URL: baseUrl },
        () => undefined,
        () => undefined,
      ),
    ).toThrow();
  }
  expect(
    uiWalkthroughInputsFromEnvironment(
      { ...COMPLETE_ENVIRONMENT, PLE_UI_WALKTHROUGH_LIVE_BASE_URL: "https://example.test" },
      () => undefined,
      () => undefined,
    ),
  ).toMatchObject({ baseUrl: "https://example.test" });
});

test("walkthrough rejects unsafe credentials and invalid seeds", () => {
  expect(() =>
    uiWalkthroughInputsFromEnvironment(
      COMPLETE_ENVIRONMENT,
      () => {
        throw new Error("symlink");
      },
      () => undefined,
    ),
  ).toThrow("unsafe metadata");
  for (const contents of ["", "student=short", "student=first\nstudent=second"]) {
    expect(() =>
      studentCredentialFromValidatedFile(
        "fixture-local-login.txt",
        () => contents,
        () => undefined,
      ),
    ).toThrow("unsafe metadata");
  }
  expect(
    studentCredentialFromValidatedFile(
      "fixture-local-login.txt",
      () => CREDENTIAL_FILE,
      () => undefined,
    ),
  ).toBe("fixture_credential_that_is_long_enough_for_the_local_provider");
  expect(
    instructorCredentialFromValidatedFile(
      "fixture-local-login.txt",
      () => CREDENTIAL_FILE,
      () => undefined,
    ),
  ).toBe("fixture_instructor_credential_that_is_long_enough_for_local_provider");
  for (const contents of [CREDENTIAL, "instructor=short", "instructor=first\ninstructor=second"]) {
    expect(() =>
      instructorCredentialFromValidatedFile(
        "fixture-local-login.txt",
        () => contents,
        () => undefined,
      ),
    ).toThrow("unsafe metadata");
  }
  for (const seed of ["-1", "1.5", "4294967296"]) {
    expect(() =>
      uiWalkthroughInputsFromEnvironment(
        { ...COMPLETE_ENVIRONMENT, PLE_UI_WALKTHROUGH_MASTER_SEED: seed },
        () => undefined,
        () => undefined,
      ),
    ).toThrow("decimal uint32");
  }
});

test("instructor-only setup accepts no arranged IDs and reads the alias only at action time", () => {
  const setupInputs = instructorSetupInputsFromEnvironment(
    {
      ...COMPLETE_ENVIRONMENT,
      PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY: "1",
      PLE_UI_WALKTHROUGH_LIVE_LEARNER_ALIAS_FILE: "fixture-learner-alias.txt",
      PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_CHECKPOINT_FILE:
        "/private/tmp/instructor-setup-checkpoint.txt",
      PLE_UI_WALKTHROUGH_CATALOG_SEARCH_TITLE:
        "Pilot retry corpus pilotref123e4567e89b12d3a456426614174000",
    },
    () => undefined,
    () => undefined,
  );
  expect(setupInputs).toMatchObject({
    baseUrl: "http://127.0.0.1:3000",
    learnerAliasFile: "fixture-learner-alias.txt",
    catalogSearchTitle: "Pilot retry corpus pilotref123e4567e89b12d3a456426614174000",
    instructorSetupCheckpointFile: "/private/tmp/instructor-setup-checkpoint.txt",
  });
  expect(
    learnerAliasFromValidatedFile(
      "fixture-learner-alias.txt",
      () => "student-local\n",
      () => undefined,
    ),
  ).toBe("student-local");
  expect(() =>
    learnerAliasFromValidatedFile(
      "fixture-learner-alias.txt",
      () => "student-local",
      () => undefined,
    ),
  ).toThrow("unsafe metadata");
  expect(() =>
    instructorSetupInputsFromEnvironment(
      { ...COMPLETE_ENVIRONMENT, PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY: "yes" },
      () => undefined,
      () => undefined,
    ),
  ).toThrow("exactly 1");
  expect(() =>
    instructorSetupInputsFromEnvironment(
      {
        ...COMPLETE_ENVIRONMENT,
        PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY: "1",
        PLE_UI_WALKTHROUGH_LIVE_LEARNER_ALIAS_FILE: "fixture-learner-alias.txt",
        PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_CHECKPOINT_FILE:
          "/private/tmp/instructor-setup-checkpoint.txt",
        PLE_UI_WALKTHROUGH_CATALOG_SEARCH_TITLE:
          "Pilot retry corpus 123e4567-e89b-12d3-a456-426614174000",
      },
      () => undefined,
      () => undefined,
    ),
  ).toThrow("bounded public corpus title");
  expect(() =>
    instructorSetupInputsFromEnvironment(
      {
        ...COMPLETE_ENVIRONMENT,
        PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY: "1",
        PLE_UI_WALKTHROUGH_LIVE_LEARNER_ALIAS_FILE: "fixture-learner-alias.txt",
        PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_CHECKPOINT_FILE: "/private/tmp/other.txt",
        PLE_UI_WALKTHROUGH_CATALOG_SEARCH_TITLE:
          "Pilot retry corpus pilotref123e4567e89b12d3a456426614174000",
      },
      () => undefined,
      () => undefined,
    ),
  ).toThrow("checkpoint must remain");
});

test("walkthrough artifact output is derived beside validated private state only", () => {
  const inputs = uiWalkthroughInputsFromEnvironment(
    { ...COMPLETE_ENVIRONMENT, PLE_UI_WALKTHROUGH_ARTIFACT_DIR: "../../test-results" },
    () => undefined,
    () => undefined,
  );
  expect(inputs?.journeyArtifactsDirectory).toBe("/private/tmp/journey-artifacts");
  expect(outputDirectoryForUiWalkthrough(inputs)).toBe("/private/tmp/journey-artifacts");
  expect(outputDirectoryForUiWalkthrough(undefined)).toBe("test-results");
  expect(() =>
    uiWalkthroughInputsFromEnvironment(
      COMPLETE_ENVIRONMENT,
      () => undefined,
      () => {
        throw new Error("unsafe");
      },
    ),
  ).toThrow("unsafe metadata");
});

test("mock selection disables for either live mode and refuses simultaneous live modes", () => {
  expect(mockPreviewServerEnabled({})).toBe(true);
  expect(mockPreviewServerEnabled({ PLE_UI_WALKTHROUGH_LIVE_REQUIRED: "1" })).toBe(false);
  expect(mockPreviewServerEnabled({ PLE_WEBWORK_LIVE_REQUIRED: "1" })).toBe(false);
  expect(() => mockPreviewServerEnabled({ PLE_UI_WALKTHROUGH_LIVE_REQUIRED: "yes" })).toThrow(
    "exactly 1",
  );
  expect(() =>
    liveModeActivationFromEnvironment({
      PLE_WEBWORK_LIVE_REQUIRED: "1",
      PLE_UI_WALKTHROUGH_LIVE_REQUIRED: "1",
    }),
  ).toThrow("cannot be enabled together");
});
