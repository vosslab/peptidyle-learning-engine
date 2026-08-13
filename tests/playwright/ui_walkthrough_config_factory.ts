// ui_walkthrough_config_factory.ts - explicit private configuration for the live UI walkthrough.

import { lstatSync, readFileSync } from "node:fs";
import { dirname, isAbsolute, relative, resolve } from "node:path";

import { defineConfig, type PlaywrightTestConfig } from "@playwright/test";

interface SharedWalkthroughInputs {
  readonly baseUrl: string;
  readonly credentialFile: string;
  readonly journeyStateFile: string;
  readonly masterSeed: number;
  readonly masterSeedText: string;
  readonly journeyArtifactsDirectory: string;
  readonly screenshotDirectory?: string;
}

export interface UiWalkthroughInputs extends SharedWalkthroughInputs {
  readonly stage: "learner_journey";
  readonly courseId: string;
  readonly masteryAssignmentId: string;
  readonly j1CheckpointFile: string;
  readonly j2CheckpointFile: string;
}

export interface InstructorSetupInputs extends SharedWalkthroughInputs {
  readonly stage: "instructor_setup";
  readonly instructorSetupCheckpointFile: string;
  readonly catalogDisplayIds: readonly [string, string, string, string];
}

export type ValidatedUiWalkthroughInputs = UiWalkthroughInputs | InstructorSetupInputs;

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu;
const DISPLAY_ID = /^P-[1-9][0-9]*-v[1-9][0-9]*$/u;
const SCREENSHOT_DIRECTORY_PARENT = "/private/tmp";
const SCREENSHOT_DIRECTORY_PREFIX = "ple-docs-screenshots.";

function objectValue(value: unknown, error: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(error);
  return value as Record<string, unknown>;
}

function exactKeys(value: Record<string, unknown>, keys: readonly string[], error: string): void {
  const actual = Object.keys(value);
  if (actual.length !== keys.length || keys.some((key, index) => actual[index] !== key)) {
    throw new Error(error);
  }
}

function stringValue(value: unknown, error: string): string {
  if (typeof value !== "string" || value === "") throw new Error(error);
  return value;
}

function inspectPrivateFile(path: string, error: string): void {
  if (!isAbsolute(path)) throw new Error(error);
  const metadata = lstatSync(path);
  const parent = lstatSync(dirname(path));
  if (
    !metadata.isFile() ||
    metadata.isSymbolicLink() ||
    (metadata.mode & 0o777) !== 0o600 ||
    !parent.isDirectory() ||
    parent.isSymbolicLink() ||
    (parent.mode & 0o777) !== 0o700
  ) {
    throw new Error(error);
  }
}

function inspectCredentialFile(path: string): void {
  const metadata = lstatSync(path);
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new Error("walkthrough credential path must be a regular non-symlink file");
  }
  if (process.platform !== "win32" && (metadata.mode & 0o777) !== 0o600) {
    throw new Error("walkthrough credential file must have exact mode 0600");
  }
}

function validatedBaseUrl(value: unknown): string {
  const urlText = stringValue(value, "walkthrough base URL must be a valid URL");
  let parsed: URL;
  try {
    parsed = new URL(urlText);
  } catch {
    throw new Error("walkthrough base URL must be a valid URL");
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new Error("walkthrough base URL must use http or https");
  }
  if (
    parsed.username !== "" ||
    parsed.password !== "" ||
    parsed.search !== "" ||
    parsed.hash !== "" ||
    parsed.pathname !== "/"
  ) {
    throw new Error(
      "walkthrough base URL must be an origin without credentials, path, query, or fragment",
    );
  }
  if (
    parsed.protocol === "http:" &&
    parsed.hostname !== "127.0.0.1" &&
    parsed.hostname !== "localhost"
  ) {
    throw new Error("walkthrough base URL allows http only for a loopback host");
  }
  return parsed.origin;
}

function validatedMasterSeed(
  value: unknown,
): Pick<SharedWalkthroughInputs, "masterSeed" | "masterSeedText"> {
  if (!Number.isInteger(value) || typeof value !== "number" || value < 0 || value > 0xffffffff) {
    throw new Error("walkthrough master seed must be a decimal uint32");
  }
  return { masterSeed: value, masterSeedText: String(value) };
}

function validatedUuid(value: unknown, error: string): string {
  const identifier = stringValue(value, error);
  if (!UUID.test(identifier)) throw new Error(error);
  return identifier;
}

function validatedChildFile(
  parentFile: string,
  actualPath: unknown,
  filename: string,
  error: string,
): string {
  const candidate = stringValue(actualPath, error);
  const expected = resolve(dirname(parentFile), filename);
  if (resolve(candidate) !== expected || relative(dirname(parentFile), expected) !== filename) {
    throw new Error(error);
  }
  return expected;
}

function journeyArtifactsDirectory(journeyStateFile: string): string {
  const parent = dirname(resolve(journeyStateFile));
  const artifacts = resolve(parent, "journey-artifacts");
  if (relative(parent, artifacts) !== "journey-artifacts") {
    throw new Error("walkthrough artifact path must remain beside private state");
  }
  return artifacts;
}

function validatedScreenshotDirectory(value: unknown): string | undefined {
  if (value === null) return undefined;
  const directory = stringValue(value, "walkthrough screenshot directory is invalid");
  if (
    !isAbsolute(directory) ||
    dirname(directory) !== SCREENSHOT_DIRECTORY_PARENT ||
    !resolve(directory).startsWith(`${SCREENSHOT_DIRECTORY_PARENT}/`) ||
    !directory.split("/")[directory.split("/").length - 1]?.startsWith(SCREENSHOT_DIRECTORY_PREFIX)
  ) {
    throw new Error("walkthrough screenshot directory must be runner-created under /private/tmp");
  }
  const metadata = lstatSync(directory);
  const getuid = process.getuid;
  if (
    !metadata.isDirectory() ||
    metadata.isSymbolicLink() ||
    getuid === undefined ||
    metadata.uid !== getuid() ||
    (metadata.mode & 0o777) !== 0o700
  ) {
    throw new Error("walkthrough screenshot directory has unsafe metadata");
  }
  return directory;
}

function validatedCatalogDisplayIds(value: unknown): InstructorSetupInputs["catalogDisplayIds"] {
  if (!Array.isArray(value) || value.length !== 4) {
    throw new Error("walkthrough catalog display IDs must be four exact public IDs");
  }
  const displayIds = value.map((candidate) => {
    const displayId = stringValue(candidate, "walkthrough catalog display ID is invalid");
    if (!DISPLAY_ID.test(displayId)) throw new Error("walkthrough catalog display ID is invalid");
    return displayId;
  });
  if (new Set(displayIds).size !== 4) {
    throw new Error("walkthrough catalog display IDs must be distinct");
  }
  const first = displayIds[0];
  const second = displayIds[1];
  const third = displayIds[2];
  const fourth = displayIds[3];
  if (first === undefined || second === undefined || third === undefined || fourth === undefined) {
    throw new Error("walkthrough catalog display IDs must be four exact public IDs");
  }
  return [first, second, third, fourth];
}

function sharedInputs(value: Record<string, unknown>): SharedWalkthroughInputs {
  const journeyStateFile = stringValue(
    value["journeyStateFile"],
    "walkthrough journey state path is invalid",
  );
  inspectPrivateFile(journeyStateFile, "walkthrough journey state path has unsafe metadata");
  const credentialFile = stringValue(
    value["credentialFile"],
    "walkthrough credential file is invalid",
  );
  try {
    inspectCredentialFile(credentialFile);
  } catch {
    throw new Error("walkthrough credential file is unreadable or has unsafe metadata");
  }
  return {
    baseUrl: validatedBaseUrl(value["baseUrl"]),
    credentialFile,
    journeyStateFile,
    ...validatedMasterSeed(value["masterSeed"]),
    journeyArtifactsDirectory: journeyArtifactsDirectory(journeyStateFile),
    screenshotDirectory: validatedScreenshotDirectory(value["screenshotDirectory"]),
  };
}

function parseInputText(inputPath: string): Record<string, unknown> {
  inspectPrivateFile(inputPath, "walkthrough inputs file has unsafe metadata");
  const bytes = readFileSync(inputPath);
  if (bytes.some((byte) => byte > 0x7f))
    throw new Error("walkthrough inputs file must be ASCII JSON");
  const text = bytes.toString("ascii");
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    throw new Error("walkthrough inputs file must be valid JSON");
  }
  if (JSON.stringify(value) !== text)
    throw new Error("walkthrough inputs file must use canonical JSON");
  return objectValue(value, "walkthrough inputs file must contain an object");
}

/** Reads the single private config boundary before Playwright creates Chromium. */
export function readUiWalkthroughInputs(inputPath: string): ValidatedUiWalkthroughInputs {
  const value = parseInputText(inputPath);
  if (value["schemaVersion"] !== 1) throw new Error("walkthrough inputs schema version is invalid");
  const stage = value["stage"];
  if (stage === "learner_journey") {
    exactKeys(
      value,
      [
        "schemaVersion",
        "stage",
        "baseUrl",
        "masterSeed",
        "credentialFile",
        "journeyStateFile",
        "j1CheckpointFile",
        "j2CheckpointFile",
        "courseId",
        "masteryAssignmentId",
        "screenshotDirectory",
      ],
      "walkthrough learner inputs have an invalid schema",
    );
    const shared = sharedInputs(value);
    return {
      stage,
      ...shared,
      j1CheckpointFile: validatedChildFile(
        shared.journeyStateFile,
        value["j1CheckpointFile"],
        "j1-checkpoint.txt",
        "walkthrough J1 checkpoint must remain beside private state",
      ),
      j2CheckpointFile: validatedChildFile(
        shared.journeyStateFile,
        value["j2CheckpointFile"],
        "j2-checkpoint.txt",
        "walkthrough J2 checkpoint must remain beside private state",
      ),
      courseId: validatedUuid(value["courseId"], "walkthrough course ID is invalid"),
      masteryAssignmentId: validatedUuid(
        value["masteryAssignmentId"],
        "walkthrough mastery assignment ID is invalid",
      ),
    };
  }
  if (stage === "instructor_setup") {
    exactKeys(
      value,
      [
        "schemaVersion",
        "stage",
        "baseUrl",
        "masterSeed",
        "credentialFile",
        "journeyStateFile",
        "instructorSetupCheckpointFile",
        "catalogDisplayIds",
        "screenshotDirectory",
      ],
      "walkthrough instructor inputs have an invalid schema",
    );
    const shared = sharedInputs(value);
    return {
      stage,
      ...shared,
      instructorSetupCheckpointFile: validatedChildFile(
        shared.journeyStateFile,
        value["instructorSetupCheckpointFile"],
        "instructor-setup-checkpoint.txt",
        "walkthrough instructor setup checkpoint must remain beside private state",
      ),
      catalogDisplayIds: validatedCatalogDisplayIds(value["catalogDisplayIds"]),
    };
  }
  throw new Error("walkthrough inputs stage is invalid");
}

/** Creates the only live UI config from explicit runner-owned paths. */
export function createUiWalkthroughConfig(
  inputPath: string,
  testDirectory: string,
): PlaywrightTestConfig {
  if (!isAbsolute(testDirectory)) {
    throw new Error("walkthrough Playwright test directory must be absolute");
  }
  const inputs = readUiWalkthroughInputs(inputPath);
  return defineConfig({
    testDir: testDirectory,
    testIgnore: ["**/_temp*", "**/dist_*/**"],
    timeout: 30_000,
    fullyParallel: true,
    reporter: "list",
    outputDir: inputs.journeyArtifactsDirectory,
    use: {
      baseURL: inputs.baseUrl,
      headless: true,
    },
    projects: [
      {
        name: "ui-walkthrough",
        metadata: { uiWalkthroughInputPath: inputPath },
      },
    ],
  });
}

/** Reads a role value only at the visible local-login action boundary. */
export function credentialFromValidatedFile(path: string, role: "student" | "instructor"): string {
  try {
    inspectCredentialFile(path);
    const contents = readFileSync(path, "utf8");
    const matches = [...contents.matchAll(new RegExp(`^${role}=([^\\r\\n]+)$`, "gmu"))];
    const credential = matches[0]?.[1];
    if (
      matches.length !== 1 ||
      credential === undefined ||
      !/^[A-Za-z0-9_-]{32,}$/u.test(credential)
    ) {
      throw new Error("invalid credential");
    }
    return credential;
  } catch {
    throw new Error("walkthrough credential file is unreadable or has unsafe metadata");
  }
}
