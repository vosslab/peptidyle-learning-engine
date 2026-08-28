// Strict private input parser shared by every independent production browser scenario.
import { lstatSync, readFileSync } from "node:fs";

export type Environment = Readonly<Record<string, string | undefined>>;
export interface BrowserScenarioInputV1 {
  readonly schemaVersion: 2;
  readonly scenarioId: string;
  readonly namespace: string;
  readonly baseUrl: string;
  readonly personas: readonly string[];
  readonly baselineReads: readonly string[];
  readonly visibleObservation: string;
  readonly serviceReceipt?: string;
  readonly faultTransition?: "gateway_submit_outage" | "deterministic_grader_exception";
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
type Value = Record<string, unknown>;
const ID = /^[a-z][a-z0-9_]{0,95}$/u;
const SERVICE_RECEIPTS = new Set(["renderer_delivery", "worker_completion"]);
const INPUT_MAXIMUM_BYTES = 16_384;
const SCREENSHOT_ARTIFACT_MAXIMUM = 64;

function required(env: Environment, key: string): string {
  const value = env[key];
  if (value === undefined || value.trim() === "")
    throw new Error(`PLE_LIVE_DEMO_BROWSER_REQUIRED=1 requires ${key}`);
  return value.trim();
}
function inspect(path: string): void {
  const stat = lstatSync(path);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > INPUT_MAXIMUM_BYTES)
    throw new Error("browser-suite input must be a small regular file");
  if (process.platform !== "win32" && (stat.mode & 0o777) !== 0o600)
    throw new Error("browser-suite input must have exact mode 0600");
}
function record(value: unknown): value is Value {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
function identifier(value: unknown): value is string {
  return typeof value === "string" && ID.test(value);
}
function ascii(value: string): boolean {
  return Array.from(value).every((character) => character.codePointAt(0)! <= 0x7f);
}
function visibleObservation(value: unknown): value is string {
  return typeof value === "string" && value.trim() !== "" && ascii(value);
}
function identifiers(value: unknown): value is string[] {
  return (
    Array.isArray(value) &&
    value.length > 0 &&
    new Set(value).size === value.length &&
    value.every(identifier)
  );
}
function exactOrigin(value: unknown): value is string {
  if (typeof value !== "string") return false;
  const url = new URL(value);
  return (
    url.protocol === "https:" &&
    url.hostname === "localhost" &&
    url.port !== "" &&
    url.pathname === "/" &&
    url.toString() === value
  );
}
function screenshotCapture(value: unknown): value is ScreenshotCapture {
  if (
    !record(value) ||
    Object.keys(value).sort().join(",") !== "artifacts,version" ||
    value.version !== 1 ||
    !Array.isArray(value.artifacts) ||
    value.artifacts.length === 0 ||
    value.artifacts.length > SCREENSHOT_ARTIFACT_MAXIMUM
  )
    return false;
  const artifactIds: string[] = [];
  for (const artifact of value.artifacts) {
    if (
      !record(artifact) ||
      Object.keys(artifact).sort().join(",") !== "artifactId,stateId" ||
      !identifier(artifact.artifactId) ||
      !identifier(artifact.stateId)
    )
      return false;
    artifactIds.push(artifact.artifactId);
  }
  return new Set(artifactIds).size === artifactIds.length;
}
function parse(contents: string): BrowserScenarioInputV1 {
  if (!ascii(contents)) throw new Error("browser-suite input must use canonical ASCII JSON");
  let input: unknown;
  try {
    input = JSON.parse(contents);
  } catch {
    throw new Error("browser-suite input is not valid JSON");
  }
  if (!record(input)) throw new Error("browser-suite input has an invalid shape");
  const expected = [
    "baseUrl",
    "baselineReads",
    "namespace",
    "personas",
    "scenarioId",
    "schemaVersion",
    "visibleObservation",
  ];
  if (input.serviceReceipt !== undefined) expected.push("serviceReceipt");
  if (input.faultTransition !== undefined) expected.push("faultTransition");
  if (input.screenshotCapture !== undefined) expected.push("screenshotCapture");
  if (
    Object.keys(input).sort().join(",") !== expected.sort().join(",") ||
    input.schemaVersion !== 2 ||
    !identifier(input.scenarioId) ||
    typeof input.namespace !== "string" ||
    !new RegExp(`^bs1-[0-9a-f]{12}-${input.scenarioId}$`, "u").test(input.namespace) ||
    !exactOrigin(input.baseUrl) ||
    !identifiers(input.personas) ||
    !identifiers(input.baselineReads) ||
    !visibleObservation(input.visibleObservation) ||
    (input.serviceReceipt !== undefined &&
      (typeof input.serviceReceipt !== "string" || !SERVICE_RECEIPTS.has(input.serviceReceipt))) ||
    (input.faultTransition !== undefined &&
      input.faultTransition !== "gateway_submit_outage" &&
      input.faultTransition !== "deterministic_grader_exception") ||
    (input.screenshotCapture !== undefined && !screenshotCapture(input.screenshotCapture))
  )
    throw new Error("browser-suite input has an invalid shape");
  const result: BrowserScenarioInputV1 = {
    schemaVersion: 2,
    scenarioId: input.scenarioId,
    namespace: input.namespace,
    baseUrl: input.baseUrl,
    personas: input.personas,
    baselineReads: input.baselineReads,
    visibleObservation: input.visibleObservation,
    ...(input.serviceReceipt === undefined ? {} : { serviceReceipt: input.serviceReceipt }),
    ...(input.faultTransition === undefined ? {} : { faultTransition: input.faultTransition }),
    ...(input.screenshotCapture === undefined
      ? {}
      : { screenshotCapture: input.screenshotCapture }),
  };
  if (JSON.stringify(result) !== contents)
    throw new Error("browser-suite input must use canonical ASCII JSON");
  return result;
}
export function liveDemoInputsFromEnvironment(
  env: Environment,
  read = (path: string): string => readFileSync(path, "utf8"),
  inspectFile = inspect,
): BrowserScenarioInputV1 | undefined {
  const active = env.PLE_LIVE_DEMO_BROWSER_REQUIRED;
  if (active === undefined || active === "" || active === "0") return undefined;
  if (active !== "1") throw new Error("PLE_LIVE_DEMO_BROWSER_REQUIRED must be exactly 1 when set");
  const path = required(env, "PLE_LIVE_DEMO_BROWSER_INPUT_FILE");
  inspectFile(path);
  return parse(read(path));
}
export function liveDemoOriginReceiptPathFromEnvironment(env: Environment): string {
  const path = required(env, "PLE_LIVE_DEMO_BROWSER_ORIGIN_RECEIPT_FILE");
  if (!path.startsWith("/"))
    throw new Error("browser-suite origin receipt must use an absolute private path");
  return path;
}
