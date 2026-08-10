// webwork_live_config.ts - fail-closed inputs for the opt-in private browser gate.

import { lstatSync, readFileSync } from "node:fs";

export interface LiveWebworkInputs {
  readonly baseUrl: string;
  readonly studentCredential: string;
  readonly assignmentId: string;
}

export type Environment = Readonly<Record<string, string | undefined>>;
export type CredentialReader = (path: string) => string;
export type CredentialInspector = (path: string) => void;

function requiredEnvironmentValue(environment: Environment, name: string): string {
  const value = environment[name];
  if (value === undefined || value.trim() === "") {
    throw new Error(`PLE_WEBWORK_LIVE_REQUIRED=1 requires ${name}`);
  }
  return value.trim();
}

function localStudentCredential(contents: string): string {
  const matches = [...contents.matchAll(/^student=([^\r\n]+)$/gmu)];
  if (matches.length !== 1 || matches[0]?.[1] === undefined) {
    throw new Error(
      "live student credential file must contain exactly one student=<credential> line",
    );
  }
  const credential = matches[0][1];
  if (!/^[A-Za-z0-9_-]{32,}$/u.test(credential)) {
    throw new Error("live student credential has an invalid local-development shape");
  }
  return credential;
}

function inspectCredentialFile(path: string): void {
  const metadata = lstatSync(path);
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new Error("live student credential path must be a regular non-symlink file");
  }
  if (process.platform !== "win32" && (metadata.mode & 0o777) !== 0o600) {
    throw new Error("live student credential file must have exact mode 0600");
  }
}

/**
 * Returns undefined only for the normal offline suite. Required mode validates
 * every live input before the Playwright configuration can create a browser.
 */
export function liveInputsFromEnvironment(
  environment: Environment,
  readCredential: CredentialReader = (path) => readFileSync(path, "utf8"),
  inspectCredential: CredentialInspector = inspectCredentialFile,
): LiveWebworkInputs | undefined {
  const required = environment["PLE_WEBWORK_LIVE_REQUIRED"];
  if (required === undefined || required === "" || required === "0") return undefined;
  if (required !== "1") {
    throw new Error("PLE_WEBWORK_LIVE_REQUIRED must be exactly 1 when set");
  }

  const baseUrl = requiredEnvironmentValue(environment, "PLE_WEBWORK_LIVE_BASE_URL");
  const parsedBaseUrl = new URL(baseUrl);
  if (parsedBaseUrl.protocol !== "http:" && parsedBaseUrl.protocol !== "https:") {
    throw new Error("PLE_WEBWORK_LIVE_BASE_URL must use http or https");
  }
  if (
    parsedBaseUrl.username !== "" ||
    parsedBaseUrl.password !== "" ||
    parsedBaseUrl.search !== ""
  ) {
    throw new Error("PLE_WEBWORK_LIVE_BASE_URL must not contain credentials or a query string");
  }

  const assignmentId = requiredEnvironmentValue(environment, "PLE_WEBWORK_LIVE_ASSIGNMENT_ID");
  if (
    !/^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/iu.test(
      assignmentId,
    )
  ) {
    throw new Error("PLE_WEBWORK_LIVE_ASSIGNMENT_ID must be a UUID");
  }
  const credentialPath = requiredEnvironmentValue(
    environment,
    "PLE_WEBWORK_LIVE_STUDENT_CREDENTIAL_FILE",
  );
  let credentialContents: string;
  try {
    inspectCredential(credentialPath);
    credentialContents = readCredential(credentialPath);
  } catch {
    throw new Error("live student credential file is unreadable or has unsafe metadata");
  }
  const studentCredential = localStudentCredential(credentialContents);
  return { baseUrl: parsedBaseUrl.toString(), studentCredential, assignmentId };
}
