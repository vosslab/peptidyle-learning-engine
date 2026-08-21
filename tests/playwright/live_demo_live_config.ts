// live_demo_live_config.ts - closed private input for the connected ordinary-site demo lane.

import { lstatSync, readFileSync } from "node:fs";

export type Environment = Readonly<Record<string, string | undefined>>;

export interface LiveDemoInputs {
  readonly baseUrl: string;
  readonly sysadminOwnershipProof: string;
}

type FileReader = (path: string) => string;
type FileInspector = (path: string) => void;

function required(environment: Environment, name: string): string {
  const value = environment[name];
  if (value === undefined || value.trim() === "") {
    throw new Error(`PLE_LIVE_DEMO_BROWSER_REQUIRED=1 requires ${name}`);
  }
  return value.trim();
}

function inspectInput(path: string): void {
  const metadata = lstatSync(path);
  if (!metadata.isFile() || metadata.isSymbolicLink()) {
    throw new Error("live-demo browser input must be a regular non-symlink file");
  }
  if (process.platform !== "win32" && (metadata.mode & 0o777) !== 0o600) {
    throw new Error("live-demo browser input must have exact mode 0600");
  }
  if (typeof process.getuid === "function" && metadata.uid !== process.getuid()) {
    throw new Error("live-demo browser input must be owned by the current user");
  }
  if (metadata.size > 256) throw new Error("live-demo browser input is too large");
}

function inputShape(contents: string): LiveDemoInputs {
  let value: unknown;
  try {
    value = JSON.parse(contents);
  } catch {
    throw new Error("live-demo browser input is not valid JSON");
  }
  if (!isRecord(value)) {
    throw new Error("live-demo browser input has an invalid shape");
  }
  const record = value;
  const keys = Object.keys(record).sort();
  if (keys.join(",") !== "baseUrl,schemaVersion,sysadminOwnershipProof") {
    throw new Error("live-demo browser input has an invalid shape");
  }
  if (
    record["schemaVersion"] !== 1 ||
    typeof record["baseUrl"] !== "string" ||
    typeof record["sysadminOwnershipProof"] !== "string"
  ) {
    throw new Error("live-demo browser input has an invalid shape");
  }
  const parsed = new URL(record["baseUrl"]);
  if (
    parsed.protocol !== "https:" ||
    parsed.hostname !== "localhost" ||
    parsed.port === "" ||
    parsed.username !== "" ||
    parsed.password !== "" ||
    parsed.search !== "" ||
    parsed.hash !== "" ||
    parsed.pathname !== "/"
  ) {
    throw new Error("live-demo browser baseUrl must be an exact localhost HTTPS origin root");
  }
  const baseUrl = parsed.toString();
  if (baseUrl !== record["baseUrl"]) {
    throw new Error("live-demo browser baseUrl must be canonical");
  }
  const sysadminOwnershipProof = record["sysadminOwnershipProof"];
  if (!/^[A-Za-z0-9_-]{43}$/u.test(sysadminOwnershipProof)) {
    throw new Error("live-demo browser proof has an invalid shape");
  }
  const decodedProof = Buffer.from(sysadminOwnershipProof, "base64url");
  if (decodedProof.length !== 32 || decodedProof.toString("base64url") !== sysadminOwnershipProof) {
    throw new Error("live-demo browser proof has an invalid shape");
  }
  const canonical = JSON.stringify({
    schemaVersion: 1,
    baseUrl,
    sysadminOwnershipProof,
  });
  if (contents !== canonical) {
    throw new Error("live-demo browser input must use canonical ASCII JSON");
  }
  return { baseUrl, sysadminOwnershipProof };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** Reads the proof only for the private test process; callers must never persist it in browser state. */
export function liveDemoInputsFromEnvironment(
  environment: Environment,
  readInput: FileReader = (path) => readFileSync(path, "utf8"),
  inspect: FileInspector = inspectInput,
): LiveDemoInputs | undefined {
  const requiredMode = environment["PLE_LIVE_DEMO_BROWSER_REQUIRED"];
  if (requiredMode === undefined || requiredMode === "" || requiredMode === "0") return undefined;
  if (requiredMode !== "1")
    throw new Error("PLE_LIVE_DEMO_BROWSER_REQUIRED must be exactly 1 when set");
  const path = required(environment, "PLE_LIVE_DEMO_BROWSER_INPUT_FILE");
  try {
    inspect(path);
    return inputShape(readInput(path));
  } catch {
    throw new Error("live-demo browser input is unreadable or unsafe");
  }
}
