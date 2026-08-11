// ui_walkthrough_live_config.ts - fail-closed inputs for the public gateway smoke.

import { lstatSync, readFileSync } from "node:fs";
import { dirname, isAbsolute, relative, resolve } from "node:path";

export interface UiWalkthroughLiveInputs {
  readonly baseUrl: string;
  readonly credentialFile: string;
  readonly courseId: string;
  readonly masteryAssignmentId: string;
  readonly masteryProblemId: string;
  /** Historical J4-only input; the instructor-created pilot path has no arranged Exam. */
  readonly examAssignmentId?: string;
  readonly masterSeed: number;
  readonly masterSeedText: string;
  readonly journeyStateFile: string;
  readonly j1CheckpointFile: string;
  readonly journeyArtifactsDirectory: string;
}

/** Inputs for the fixed instructor-only child before public IDs exist. */
export interface InstructorSetupLiveInputs {
  readonly baseUrl: string;
  readonly credentialFile: string;
  readonly learnerAliasFile: string;
  /** Bounded public title of the single fresh API-arranged catalog entry. */
  readonly catalogSearchTitle: string;
  readonly masterSeed: number;
  readonly masterSeedText: string;
  readonly journeyStateFile: string;
  readonly instructorSetupCheckpointFile: string;
  readonly journeyArtifactsDirectory: string;
}

export type Environment = Readonly<Record<string, string | undefined>>;
export type CredentialReader = (path: string) => string;
export type CredentialInspector = (path: string) => void;
export type JourneyStateInspector = (path: string) => void;
export type AliasReader = (path: string) => string;

function requiredEnvironmentValue(environment: Environment, name: string): string {
  const value = environment[name];
  if (value === undefined || value.trim() === "") {
    throw new Error(`PLE_UI_WALKTHROUGH_LIVE_REQUIRED=1 requires ${name}`);
  }
  return value.trim();
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

function inspectJourneyStateFile(path: string): void {
  if (!isAbsolute(path)) throw new Error("walkthrough journey state path must be absolute");
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
    throw new Error("walkthrough journey state path has unsafe metadata");
  }
}

export function walkthroughArtifactsDirectory(journeyStateFile: string): string {
  const parent = dirname(resolve(journeyStateFile));
  const artifacts = resolve(parent, "journey-artifacts");
  const pathFromParent = relative(parent, artifacts);
  if (pathFromParent !== "journey-artifacts") {
    throw new Error("walkthrough artifact path must remain beside private state");
  }
  return artifacts;
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu;
const CATALOG_SEARCH_TITLE = new RegExp("^Pilot retry corpus pilotref[0-9a-f]{32}$", "u");

function validateLocalCredential(contents: string, role: "student" | "instructor"): string {
  const matches = [...contents.matchAll(new RegExp(`^${role}=([^\\r\\n]+)$`, "gmu"))];
  if (matches.length !== 1 || matches[0]?.[1] === undefined) {
    throw new Error(
      `walkthrough credential file must contain exactly one ${role}=<credential> line`,
    );
  }
  const credential = matches[0][1];
  if (!/^[A-Za-z0-9_-]{32,}$/u.test(credential)) {
    throw new Error("walkthrough student credential has an invalid local-development shape");
  }
  return credential;
}

/** Reads the student value only at the visible local-login action boundary. */
export function studentCredentialFromValidatedFile(
  path: string,
  readCredential: CredentialReader = (credentialPath) => readFileSync(credentialPath, "utf8"),
  inspectCredential: CredentialInspector = inspectCredentialFile,
): string {
  try {
    inspectCredential(path);
    const credential = validateLocalCredential(readCredential(path), "student");
    return credential;
  } catch {
    throw new Error("walkthrough credential file is unreadable or has unsafe metadata");
  }
}

/** Reads the instructor value only at the visible local-login action boundary. */
export function instructorCredentialFromValidatedFile(
  path: string,
  readCredential: CredentialReader = (credentialPath) => readFileSync(credentialPath, "utf8"),
  inspectCredential: CredentialInspector = inspectCredentialFile,
): string {
  try {
    inspectCredential(path);
    const credential = validateLocalCredential(readCredential(path), "instructor");
    return credential;
  } catch {
    throw new Error("walkthrough credential file is unreadable or has unsafe metadata");
  }
}

/** Reads the configured learner alias only when J12 fills its visible local-only form. */
export function learnerAliasFromValidatedFile(
  path: string,
  readAlias: AliasReader = (aliasPath) => readFileSync(aliasPath, "ascii"),
  inspectAlias: CredentialInspector = inspectCredentialFile,
): string {
  try {
    inspectAlias(path);
    const alias = readAlias(path);
    if (!/^[A-Za-z0-9][A-Za-z0-9._-]{0,127}\n$/u.test(alias)) throw new Error("invalid alias");
    return alias.slice(0, -1);
  } catch {
    throw new Error("walkthrough learner alias is unreadable or has unsafe metadata");
  }
}

function validatedBaseUrl(value: string): string {
  let parsed: URL;
  try {
    parsed = new URL(value);
  } catch {
    throw new Error("PLE_UI_WALKTHROUGH_LIVE_BASE_URL must be a valid URL");
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    throw new Error("PLE_UI_WALKTHROUGH_LIVE_BASE_URL must use http or https");
  }
  if (
    parsed.username !== "" ||
    parsed.password !== "" ||
    parsed.search !== "" ||
    parsed.hash !== "" ||
    parsed.pathname !== "/"
  ) {
    throw new Error(
      "PLE_UI_WALKTHROUGH_LIVE_BASE_URL must be an origin without credentials, " +
        "path, query, or fragment",
    );
  }
  if (
    parsed.protocol === "http:" &&
    parsed.hostname !== "127.0.0.1" &&
    parsed.hostname !== "localhost"
  ) {
    throw new Error("PLE_UI_WALKTHROUGH_LIVE_BASE_URL allows http only for a loopback host");
  }
  return parsed.origin;
}

function validatedMasterSeed(
  value: string,
): Pick<UiWalkthroughLiveInputs, "masterSeed" | "masterSeedText"> {
  if (!/^[0-9]+$/u.test(value)) {
    throw new Error("PLE_UI_WALKTHROUGH_MASTER_SEED must be a decimal uint32");
  }
  const masterSeed = Number(value);
  if (!Number.isSafeInteger(masterSeed) || masterSeed > 0xffffffff) {
    throw new Error("PLE_UI_WALKTHROUGH_MASTER_SEED must be a decimal uint32");
  }
  return { masterSeed, masterSeedText: String(masterSeed) };
}

function requiredUuid(environment: Environment, name: string): string {
  const value = requiredEnvironmentValue(environment, name);
  if (!UUID.test(value)) throw new Error(`${name} must be a UUID`);
  return value;
}

function requiredCatalogSearchTitle(environment: Environment): string {
  const value = requiredEnvironmentValue(environment, "PLE_UI_WALKTHROUGH_CATALOG_SEARCH_TITLE");
  if (!CATALOG_SEARCH_TITLE.test(value)) {
    throw new Error(
      "PLE_UI_WALKTHROUGH_CATALOG_SEARCH_TITLE must be a bounded public corpus title",
    );
  }
  return value;
}

/**
 * Returns undefined only for the normal offline suite. Required mode validates
 * every input before Playwright configuration can create a browser context.
 */
export function uiWalkthroughInputsFromEnvironment(
  environment: Environment,
  inspectCredential: CredentialInspector = inspectCredentialFile,
  inspectJourneyState: JourneyStateInspector = inspectJourneyStateFile,
): UiWalkthroughLiveInputs | undefined {
  const required = environment["PLE_UI_WALKTHROUGH_LIVE_REQUIRED"];
  if (required === undefined || required === "" || required === "0") return undefined;
  if (required !== "1") {
    throw new Error("PLE_UI_WALKTHROUGH_LIVE_REQUIRED must be exactly 1 when set");
  }

  const baseUrl = validatedBaseUrl(
    requiredEnvironmentValue(environment, "PLE_UI_WALKTHROUGH_LIVE_BASE_URL"),
  );
  const credentialPath = requiredEnvironmentValue(
    environment,
    "PLE_UI_WALKTHROUGH_LIVE_CREDENTIAL_FILE",
  );
  try {
    inspectCredential(credentialPath);
  } catch {
    throw new Error("walkthrough credential file is unreadable or has unsafe metadata");
  }
  const journeyStateFile = requiredEnvironmentValue(
    environment,
    "PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE",
  );
  try {
    inspectJourneyState(journeyStateFile);
  } catch {
    throw new Error("walkthrough journey state path is unreadable or has unsafe metadata");
  }
  const journeyArtifactsDirectory = walkthroughArtifactsDirectory(journeyStateFile);
  const j1CheckpointFile = resolve(dirname(journeyStateFile), "j1-checkpoint.txt");
  if (relative(dirname(journeyStateFile), j1CheckpointFile) !== "j1-checkpoint.txt") {
    throw new Error("walkthrough J1 checkpoint path must remain beside private state");
  }
  return {
    baseUrl,
    credentialFile: credentialPath,
    courseId: requiredUuid(environment, "PLE_UI_WALKTHROUGH_LIVE_COURSE_ID"),
    masteryAssignmentId: requiredUuid(environment, "PLE_UI_WALKTHROUGH_LIVE_MASTERY_ASSIGNMENT_ID"),
    masteryProblemId: requiredUuid(environment, "PLE_UI_WALKTHROUGH_LIVE_MASTERY_PROBLEM_ID"),
    examAssignmentId:
      environment["PLE_UI_WALKTHROUGH_LIVE_EXAM_ASSIGNMENT_ID"] === undefined
        ? undefined
        : requiredUuid(environment, "PLE_UI_WALKTHROUGH_LIVE_EXAM_ASSIGNMENT_ID"),
    ...validatedMasterSeed(requiredEnvironmentValue(environment, "PLE_UI_WALKTHROUGH_MASTER_SEED")),
    journeyStateFile,
    j1CheckpointFile,
    journeyArtifactsDirectory,
  };
}

/** Parses the narrow instructor-only live boundary, before J11 can create public IDs. */
export function instructorSetupInputsFromEnvironment(
  environment: Environment,
  inspectCredential: CredentialInspector = inspectCredentialFile,
  inspectJourneyState: JourneyStateInspector = inspectJourneyStateFile,
): InstructorSetupLiveInputs | undefined {
  if (environment["PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY"] === undefined) return undefined;
  if (environment["PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY"] !== "1") {
    throw new Error("PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_ONLY must be exactly 1 when set");
  }
  if (environment["PLE_UI_WALKTHROUGH_LIVE_REQUIRED"] !== "1") {
    throw new Error("instructor setup requires PLE_UI_WALKTHROUGH_LIVE_REQUIRED=1");
  }
  const baseUrl = validatedBaseUrl(
    requiredEnvironmentValue(environment, "PLE_UI_WALKTHROUGH_LIVE_BASE_URL"),
  );
  const credentialFile = requiredEnvironmentValue(
    environment,
    "PLE_UI_WALKTHROUGH_LIVE_CREDENTIAL_FILE",
  );
  const learnerAliasFile = requiredEnvironmentValue(
    environment,
    "PLE_UI_WALKTHROUGH_LIVE_LEARNER_ALIAS_FILE",
  );
  const journeyStateFile = requiredEnvironmentValue(
    environment,
    "PLE_UI_WALKTHROUGH_JOURNEY_STATE_FILE",
  );
  const instructorSetupCheckpointFile = requiredEnvironmentValue(
    environment,
    "PLE_UI_WALKTHROUGH_INSTRUCTOR_SETUP_CHECKPOINT_FILE",
  );
  try {
    inspectCredential(credentialFile);
    inspectCredential(learnerAliasFile);
    inspectJourneyState(journeyStateFile);
  } catch {
    throw new Error("walkthrough instructor setup input has unsafe metadata");
  }
  const expectedCheckpointFile = resolve(
    dirname(journeyStateFile),
    "instructor-setup-checkpoint.txt",
  );
  if (
    resolve(instructorSetupCheckpointFile) !== expectedCheckpointFile ||
    relative(dirname(journeyStateFile), expectedCheckpointFile) !==
      "instructor-setup-checkpoint.txt"
  ) {
    throw new Error("walkthrough instructor setup checkpoint must remain beside private state");
  }
  return {
    baseUrl,
    credentialFile,
    learnerAliasFile,
    catalogSearchTitle: requiredCatalogSearchTitle(environment),
    ...validatedMasterSeed(requiredEnvironmentValue(environment, "PLE_UI_WALKTHROUGH_MASTER_SEED")),
    journeyStateFile,
    instructorSetupCheckpointFile,
    journeyArtifactsDirectory: walkthroughArtifactsDirectory(journeyStateFile),
  };
}
