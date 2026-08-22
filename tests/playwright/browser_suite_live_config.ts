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
  readonly faultTransition?: "gateway_submit_outage";
  readonly sysadminOwnershipProof?: string;
  readonly screenshotCapture?: ScreenshotCapture;
}

export interface ScreenshotCapture {
  readonly version: 1;
  readonly artifacts: readonly ScreenshotArtifact[];
}

export interface ScreenshotArtifact {
  readonly artifactId: string;
  readonly stateId: string;
}

export interface WebAuthnCredential {
  readonly credentialId: string;
  readonly isResidentCredential: true;
  readonly rpId: "localhost";
  readonly privateKey: string;
  readonly signCount: number;
  readonly userHandle: string;
  readonly backupEligibility: boolean;
  readonly backupState: boolean;
}

export interface WebAuthnContinuation {
  readonly version: 1;
  readonly origin: string;
  readonly rpId: "localhost";
  readonly credentials: readonly [WebAuthnCredential];
}

type ScenarioInputRecord = Record<string, unknown>;
type WebAuthnContinuationRecord = Record<string, unknown>;

const ID = /^[a-z][a-z0-9_]{0,95}$/u;
const PROOF = /^[A-Za-z0-9_-]{43}$/u;
const SERVICE_RECEIPTS = new Set(["renderer_delivery", "worker_completion"]);
const FAULT_TRANSITIONS = new Set(["gateway_submit_outage"]);
const REQUIREMENTS = new Set<SysadminRequirement>(["not_required", "unclaimed", "claimed"]);
const CONTINUATION_MAXIMUM_BYTES = 16_384;
const BROWSER_SUITE_INPUT_MAXIMUM_BYTES = 16_384;
const SCREENSHOT_ARTIFACT_MAXIMUM = 64;
const BASE64URL = /^[A-Za-z0-9_-]+$/u;

function required(env: Environment, key: string): string {
  const value = env[key];
  if (value === undefined || value.trim() === "") {
    throw new Error(`PLE_LIVE_DEMO_BROWSER_REQUIRED=1 requires ${key}`);
  }
  return value.trim();
}

function inspect(path: string): void {
  const stat = lstatSync(path);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > BROWSER_SUITE_INPUT_MAXIMUM_BYTES) {
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
  const expected = requiredKeys(
    record.serviceReceipt !== undefined,
    record.faultTransition !== undefined,
    record.screenshotCapture !== undefined,
    requirement,
  );
  if (Object.keys(record).sort().join(",") !== expected.join(",")) {
    throw new Error("browser-suite input has an invalid shape");
  }
}

function requiredKeys(
  hasServiceReceipt: boolean,
  hasFaultTransition: boolean,
  hasScreenshotCapture: boolean,
  requirement: SysadminRequirement,
): string[] {
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
  if (hasFaultTransition) keys.push("faultTransition");
  if (requirement === "unclaimed") keys.push("sysadminOwnershipProof");
  if (hasScreenshotCapture) keys.push("screenshotCapture");
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
  if (!validFaultTransition(record.faultTransition)) {
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

function validFaultTransition(value: unknown): value is "gateway_submit_outage" | undefined {
  return value === undefined || (typeof value === "string" && FAULT_TRANSITIONS.has(value));
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
  const faultTransition = optionalFaultTransition(record.faultTransition);
  const proof = optionalOwnershipProof(record.sysadminOwnershipProof, requirement);
  const screenshotCapture = optionalScreenshotCapture(record.screenshotCapture);
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
  const withFault =
    faultTransition === undefined ? withService : { ...withService, faultTransition };
  const withProof =
    proof === undefined ? withFault : { ...withFault, sysadminOwnershipProof: proof };
  return screenshotCapture === undefined ? withProof : { ...withProof, screenshotCapture };
}

function optionalScreenshotCapture(value: unknown): ScreenshotCapture | undefined {
  if (value === undefined) return undefined;
  if (
    !isRecord(value) ||
    value.version !== 1 ||
    !Array.isArray(value.artifacts) ||
    value.artifacts.length < 1 ||
    value.artifacts.length > SCREENSHOT_ARTIFACT_MAXIMUM
  ) {
    throw new Error("browser screenshot input has an invalid shape");
  }
  const artifacts = value.artifacts.map((artifact) => {
    if (
      !isRecord(artifact) ||
      Object.keys(artifact).sort().join(",") !== "artifactId,stateId" ||
      !validIdentifier(artifact.artifactId) ||
      !validIdentifier(artifact.stateId)
    ) {
      throw new Error("browser screenshot input has an invalid shape");
    }
    return { artifactId: artifact.artifactId, stateId: artifact.stateId };
  });
  if (new Set(artifacts.map((artifact) => artifact.artifactId)).size !== artifacts.length)
    throw new Error("browser screenshot input has an invalid shape");
  return { version: 1, artifacts };
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

function optionalFaultTransition(value: unknown): "gateway_submit_outage" | undefined {
  if (!validFaultTransition(value)) {
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

function inspectContinuation(path: string): void {
  const stat = lstatSync(path);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > CONTINUATION_MAXIMUM_BYTES) {
    throw new Error("WebAuthn continuation must be a small regular file");
  }
  if (process.platform !== "win32" && (stat.mode & 0o777) !== 0o600) {
    throw new Error("WebAuthn continuation must have exact mode 0600");
  }
  if (process.getuid !== undefined && stat.uid !== process.getuid()) {
    throw new Error("WebAuthn continuation must be owned by the invoking user");
  }
}

function parseWebAuthnContinuation(contents: string, expectedOrigin: string): WebAuthnContinuation {
  requireAscii(contents);
  const record = parseWebAuthnContinuationRecord(contents);
  if (Object.keys(record).sort().join(",") !== "credentials,origin,rpId,version") {
    throw new Error("WebAuthn continuation has an invalid shape");
  }
  if (record.version !== 1 || record.rpId !== "localhost") {
    throw new Error("WebAuthn continuation has an invalid shape");
  }
  const origin = requireContinuationOrigin(record.origin, expectedOrigin);
  if (!Array.isArray(record.credentials) || record.credentials.length !== 1) {
    throw new Error("WebAuthn continuation must contain exactly one credential");
  }
  const credential = requireWebAuthnCredential(record.credentials[0], record.rpId);
  const continuation: WebAuthnContinuation = {
    version: 1,
    origin,
    rpId: "localhost",
    credentials: [credential],
  };
  if (JSON.stringify(continuation) !== contents) {
    throw new Error("WebAuthn continuation must use canonical ASCII JSON");
  }
  return continuation;
}

function parseWebAuthnContinuationRecord(contents: string): WebAuthnContinuationRecord {
  let value: unknown;
  try {
    value = JSON.parse(contents);
  } catch {
    throw new Error("WebAuthn continuation is not valid JSON");
  }
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("WebAuthn continuation has an invalid shape");
  }
  return value as WebAuthnContinuationRecord;
}

function requireContinuationOrigin(value: unknown, expectedOrigin: string): string {
  if (typeof value !== "string") throw new Error("WebAuthn continuation has an invalid origin");
  const url = new URL(value);
  const exactOrigin =
    url.protocol === "https:" &&
    url.hostname === "localhost" &&
    url.port !== "" &&
    url.pathname === "/" &&
    url.search === "" &&
    url.hash === "" &&
    url.username === "" &&
    url.password === "" &&
    url.origin === value &&
    value === expectedOrigin;
  if (!exactOrigin)
    throw new Error("WebAuthn continuation must use an exact localhost HTTPS origin");
  return value;
}

function requireWebAuthnCredential(value: unknown, rpId: unknown): WebAuthnCredential {
  if (typeof rpId !== "string" || !isRecord(value)) {
    throw new Error("WebAuthn continuation has an invalid credential");
  }
  const keys = [
    "backupEligibility",
    "backupState",
    "credentialId",
    "isResidentCredential",
    "privateKey",
    "rpId",
    "signCount",
    "userHandle",
  ];
  if (Object.keys(value).sort().join(",") !== keys.join(",")) {
    throw new Error("WebAuthn continuation has an invalid credential");
  }
  if (
    value.isResidentCredential !== true ||
    value.rpId !== rpId ||
    !isContinuationBinary(value.credentialId, 1, 1024) ||
    !isContinuationBinary(value.privateKey, 1, 4096) ||
    !isContinuationBinary(value.userHandle, 1, 64) ||
    !isCount(value.signCount) ||
    typeof value.backupEligibility !== "boolean" ||
    typeof value.backupState !== "boolean" ||
    (value.backupState && !value.backupEligibility)
  ) {
    throw new Error("WebAuthn continuation has an invalid credential");
  }
  return {
    credentialId: value.credentialId,
    isResidentCredential: true,
    rpId: "localhost",
    privateKey: value.privateKey,
    signCount: value.signCount,
    userHandle: value.userHandle,
    backupEligibility: value.backupEligibility,
    backupState: value.backupState,
  };
}

function isContinuationBinary(
  value: unknown,
  minimumBytes: number,
  maximumBytes: number,
): value is string {
  if (typeof value !== "string" || !BASE64URL.test(value)) return false;
  const decoded = Buffer.from(value, "base64url");
  return (
    decoded.length >= minimumBytes &&
    decoded.length <= maximumBytes &&
    decoded.toString("base64url") === value
  );
}

function isCount(value: unknown): value is number {
  return typeof value === "number" && Number.isInteger(value) && value >= 0 && value <= 0xffffffff;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
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

/** Returns the one private output path reserved for the unclaimed setup producer. */
export function webAuthnContinuationPathForProducerFromEnvironment(env: Environment): string {
  const raw = env.PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_FILE;
  const path = required(env, "PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_FILE");
  if (raw !== path || !path.startsWith("/")) {
    throw new Error("WebAuthn continuation must use an exact absolute private path");
  }
  return path;
}

/** Returns the distinct owner-selected acknowledgement output path for one claimed child. */
export function webAuthnContinuationAcknowledgementPathFromEnvironment(env: Environment): string {
  const raw = env.PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_ACK_FILE;
  const path = required(env, "PLE_BROWSER_SUITE_WEBAUTHN_CONTINUATION_ACK_FILE");
  if (raw !== path || !path.startsWith("/")) {
    throw new Error(
      "WebAuthn continuation acknowledgement must use an exact absolute private path",
    );
  }
  return path;
}

/** Reads the owner-validated private continuation needed by a claimed browser child. */
export function webAuthnContinuationFromEnvironment(
  env: Environment,
  expectedOrigin: string,
  read = (path: string): string => readFileSync(path, "utf8"),
  inspectFile = inspectContinuation,
): WebAuthnContinuation {
  const path = webAuthnContinuationPathForProducerFromEnvironment(env);
  inspectFile(path);
  // ASVS 1.5.1 and 2.2.1: decode only the closed, canonical credential projection.
  return parseWebAuthnContinuation(read(path), expectedOrigin);
}

export function liveDemoOriginReceiptPathFromEnvironment(env: Environment): string {
  const path = required(env, "PLE_LIVE_DEMO_BROWSER_ORIGIN_RECEIPT_FILE");
  if (!path.startsWith("/")) {
    throw new Error("browser-suite origin receipt must use an absolute private path");
  }
  return path;
}
