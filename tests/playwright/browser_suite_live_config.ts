// Strict V2 private input parser shared by every production browser scenario.
import { lstatSync, readFileSync } from "node:fs";

export type Environment = Readonly<Record<string, string | undefined>>;
export type SysadminRequirement = "not_required" | "unclaimed" | "claimed";

export interface BrowserScenarioInputV1 {
  readonly schemaVersion: 2;
  readonly scenarioId: string;
  readonly namespace: string;
  readonly baseUrl: string;
  readonly personas: readonly string[];
  readonly baselineReads: readonly string[];
  readonly sysadminRequirement: SysadminRequirement;
  readonly visibleObservation: string;
  readonly serviceReceipt?: string;
  readonly sysadminOwnershipProof?: string;
}

type ScenarioInputRecord = Record<string, unknown>;

const ID = /^[a-z][a-z0-9_]{0,95}$/u;
const PROOF = /^[A-Za-z0-9_-]{43}$/u;
const SERVICE_RECEIPTS = new Set(["renderer_delivery", "worker_completion"]);
const REQUIREMENTS = new Set<SysadminRequirement>(["not_required", "unclaimed", "claimed"]);

function required(env: Environment, key: string): string {
  const value = env[key];
  if (value === undefined || value.trim() === "") {
    throw new Error(`PLE_LIVE_DEMO_BROWSER_REQUIRED=1 requires ${key}`);
  }
  return value.trim();
}

function inspect(path: string): void {
  const stat = lstatSync(path);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 1024) {
    throw new Error("browser-suite input must be a small regular file");
  }
  if (process.platform !== "win32" && (stat.mode & 0o777) !== 0o600) {
    throw new Error("browser-suite input must have exact mode 0600");
  }
}

function parse(contents: string): BrowserScenarioInputV1 {
  requireAscii(contents);
  const record = parseRecord(contents);
  const requirement = parseRequirement(record.sysadminRequirement);
  requireExpectedKeys(record, requirement);
  validateBaseFields(record, requirement);
  validateOrigin(record.baseUrl);
  validateProof(record, requirement);
  const canonical = canonicalInput(record, requirement);
  requireCanonicalJson(canonical, contents);
  return canonical;
}

function requireAscii(contents: string): void {
  for (const character of contents) {
    const codePoint = character.codePointAt(0);
    if (codePoint === undefined || codePoint > 0x7f) {
      throw new Error("browser-suite input must use canonical ASCII JSON");
    }
  }
}

function parseRecord(contents: string): ScenarioInputRecord {
  let value: unknown;
  try {
    value = JSON.parse(contents);
  } catch {
    throw new Error("browser-suite input is not valid JSON");
  }
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("browser-suite input has an invalid shape");
  }
  return value as ScenarioInputRecord;
}

function parseRequirement(value: unknown): SysadminRequirement {
  if (typeof value !== "string" || !REQUIREMENTS.has(value as SysadminRequirement)) {
    throw new Error("browser-suite input has an invalid shape");
  }
  return value as SysadminRequirement;
}

function requireExpectedKeys(record: ScenarioInputRecord, requirement: SysadminRequirement): void {
  const expected = requiredKeys(record.serviceReceipt !== undefined, requirement);
  if (Object.keys(record).sort().join(",") !== expected.join(",")) {
    throw new Error("browser-suite input has an invalid shape");
  }
}

function requiredKeys(hasServiceReceipt: boolean, requirement: SysadminRequirement): string[] {
  const keys = [
    "baseUrl",
    "baselineReads",
    "namespace",
    "personas",
    "scenarioId",
    "schemaVersion",
    "sysadminRequirement",
    "visibleObservation",
  ];
  if (hasServiceReceipt) keys.push("serviceReceipt");
  if (requirement === "unclaimed") keys.push("sysadminOwnershipProof");
  return keys.sort();
}

function validateBaseFields(record: ScenarioInputRecord, requirement: SysadminRequirement): void {
  if (record.schemaVersion !== 2 || !validIdentifier(record.scenarioId)) {
    throw new Error("browser-suite input has an invalid shape");
  }
  if (!validNamespace(record.namespace, record.scenarioId)) {
    throw new Error("browser-suite input has an invalid shape");
  }
  if (typeof record.baseUrl !== "string") {
    throw new Error("browser-suite input has an invalid shape");
  }
  if (!validStringList(record.personas) || !validStringList(record.baselineReads)) {
    throw new Error("browser-suite input has an invalid shape");
  }
  if (!validIdentifier(record.visibleObservation)) {
    throw new Error("browser-suite input has an invalid shape");
  }
  if (!validServiceReceipt(record.serviceReceipt)) {
    throw new Error("browser-suite input has an invalid shape");
  }
  if (record.sysadminRequirement !== requirement) {
    throw new Error("browser-suite input has an invalid shape");
  }
}

function validIdentifier(value: unknown): value is string {
  return typeof value === "string" && ID.test(value);
}

function validNamespace(namespace: unknown, scenarioId: unknown): namespace is string {
  return (
    typeof namespace === "string" &&
    typeof scenarioId === "string" &&
    new RegExp(`^bs1-[0-9a-f]{12}-${scenarioId}$`, "u").test(namespace)
  );
}

function validStringList(value: unknown): value is string[] {
  return (
    Array.isArray(value) &&
    value.length > 0 &&
    new Set(value).size === value.length &&
    value.every(validIdentifier)
  );
}

function validServiceReceipt(value: unknown): value is string | undefined {
  return value === undefined || (typeof value === "string" && SERVICE_RECEIPTS.has(value));
}

function validateOrigin(baseUrl: unknown): void {
  if (typeof baseUrl !== "string") {
    throw new Error("browser-suite input has an invalid shape");
  }
  const url = new URL(baseUrl);
  const isExactLocalhostRoot =
    url.protocol === "https:" &&
    url.hostname === "localhost" &&
    url.port !== "" &&
    url.pathname === "/" &&
    url.toString() === baseUrl;
  if (!isExactLocalhostRoot) {
    throw new Error("browser-suite input must use the exact localhost HTTPS origin root");
  }
}

function validateProof(record: ScenarioInputRecord, requirement: SysadminRequirement): void {
  const proof = record.sysadminOwnershipProof;
  if (requirement === "unclaimed") {
    if (!validProof(proof)) {
      throw new Error("browser-suite input has an invalid ownership proof");
    }
    return;
  }
  if (proof !== undefined) {
    throw new Error("browser-suite input proof belongs only to unclaimed scenarios");
  }
}

function validProof(value: unknown): value is string {
  return (
    typeof value === "string" &&
    PROOF.test(value) &&
    Buffer.from(value, "base64url").toString("base64url") === value
  );
}

function canonicalInput(
  record: ScenarioInputRecord,
  requirement: SysadminRequirement,
): BrowserScenarioInputV1 {
  const scenarioId = requireIdentifier(record.scenarioId);
  const namespace = requireNamespace(record.namespace, scenarioId);
  const baseUrl = requireString(record.baseUrl);
  const personas = requireStringList(record.personas);
  const baselineReads = requireStringList(record.baselineReads);
  const visibleObservation = requireIdentifier(record.visibleObservation);
  const serviceReceipt = optionalServiceReceipt(record.serviceReceipt);
  const proof = optionalOwnershipProof(record.sysadminOwnershipProof, requirement);
  const core: BrowserScenarioInputV1 = {
    schemaVersion: 2,
    scenarioId,
    namespace,
    baseUrl,
    personas,
    baselineReads,
    sysadminRequirement: requirement,
    visibleObservation,
  };
  const withService = serviceReceipt === undefined ? core : { ...core, serviceReceipt };
  return proof === undefined ? withService : { ...withService, sysadminOwnershipProof: proof };
}

function requireIdentifier(value: unknown): string {
  if (typeof value !== "string" || !ID.test(value)) {
    throw new Error("browser-suite input has an invalid shape");
  }
  return value;
}

function requireNamespace(namespace: unknown, scenarioId: string): string {
  if (
    typeof namespace !== "string" ||
    !new RegExp(`^bs1-[0-9a-f]{12}-${scenarioId}$`, "u").test(namespace)
  ) {
    throw new Error("browser-suite input has an invalid shape");
  }
  return namespace;
}

function requireString(value: unknown): string {
  if (typeof value !== "string") {
    throw new Error("browser-suite input has an invalid shape");
  }
  return value;
}

function requireStringList(value: unknown): readonly string[] {
  if (!validStringList(value)) {
    throw new Error("browser-suite input has an invalid shape");
  }
  return value;
}

function optionalServiceReceipt(value: unknown): string | undefined {
  if (!validServiceReceipt(value)) {
    throw new Error("browser-suite input has an invalid shape");
  }
  return value;
}

function optionalOwnershipProof(
  value: unknown,
  requirement: SysadminRequirement,
): string | undefined {
  if (requirement === "unclaimed") {
    if (!validProof(value)) {
      throw new Error("browser-suite input has an invalid ownership proof");
    }
    return value;
  }
  if (value !== undefined) {
    throw new Error("browser-suite input proof belongs only to unclaimed scenarios");
  }
  return undefined;
}

function requireCanonicalJson(input: BrowserScenarioInputV1, contents: string): void {
  if (JSON.stringify(input) !== contents) {
    throw new Error("browser-suite input must use canonical ASCII JSON");
  }
}

export function liveDemoInputsFromEnvironment(
  env: Environment,
  read = (path: string): string => readFileSync(path, "utf8"),
  inspectFile = inspect,
): BrowserScenarioInputV1 | undefined {
  const active = env.PLE_LIVE_DEMO_BROWSER_REQUIRED;
  if (active === undefined || active === "" || active === "0") return undefined;
  if (active !== "1") {
    throw new Error("PLE_LIVE_DEMO_BROWSER_REQUIRED must be exactly 1 when set");
  }
  const path = required(env, "PLE_LIVE_DEMO_BROWSER_INPUT_FILE");
  inspectFile(path);
  return parse(read(path));
}

export function liveDemoOriginReceiptPathFromEnvironment(env: Environment): string {
  const path = required(env, "PLE_LIVE_DEMO_BROWSER_ORIGIN_RECEIPT_FILE");
  if (!path.startsWith("/")) {
    throw new Error("browser-suite origin receipt must use an absolute private path");
  }
  return path;
}
