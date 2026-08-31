// Strict private hand-off for one Published Question WebWork fixture.
import { lstatSync, readFileSync, writeFileSync } from "node:fs";

export const webworkPublishedQuestionTitle =
  "Biochemistry: Identify hydrophobic compounds from formulas";

export interface WebworkPublishedQuestionFixtureInput {
  readonly schemaVersion: 1;
  readonly scenarioId: "webwork_delivery";
  readonly questionId: string;
  readonly title: typeof webworkPublishedQuestionTitle;
}

export interface VisibleIssuanceAcknowledgement {
  readonly schemaVersion: 1;
  readonly scenarioId: "webwork_delivery";
  readonly namespace: string;
  readonly event: "visible_question_issued";
}

type RecordValue = Record<string, unknown>;

const questionId = /^[A-Z0-9]{3}-[A-Z0-9]{4}$/u;
const namespace = /^bs1-[0-9a-f]{12}-webwork_delivery$/u;
const maximumBytes = 1_024;

export function requireWebworkPublishedQuestionFixtureInput(
  env: Readonly<Record<string, string | undefined>>,
): WebworkPublishedQuestionFixtureInput {
  const path = requiredPath(env, "PLE_WEBWORK_PUBLISHED_QUESTION_FIXTURE_INPUT_FILE");
  const contents = readPrivateAscii(path, "WebWork Published Question fixture input");
  const value = parseRecord(contents);
  if (
    value.schemaVersion !== 1 ||
    value.scenarioId !== "webwork_delivery" ||
    typeof value.questionId !== "string" ||
    !questionId.test(value.questionId) ||
    value.title !== webworkPublishedQuestionTitle ||
    keyList(value) !== "questionId,scenarioId,schemaVersion,title"
  ) {
    throw new Error("WebWork Published Question fixture input is invalid");
  }
  const result: WebworkPublishedQuestionFixtureInput = {
    schemaVersion: 1,
    scenarioId: "webwork_delivery",
    questionId: value.questionId,
    title: webworkPublishedQuestionTitle,
  };
  requireCanonical(contents, result);
  return result;
}

export function writeVisibleIssuanceAcknowledgement(
  env: Readonly<Record<string, string | undefined>>,
  value: WebworkPublishedQuestionFixtureInput,
  scenarioNamespace: string,
): void {
  if (value.scenarioId !== "webwork_delivery" || !namespace.test(scenarioNamespace)) {
    throw new Error("WebWork visible issuance acknowledgement is invalid");
  }
  const path = requiredPath(env, "PLE_WEBWORK_RENDERER_ISSUANCE_ACK_FILE");
  const acknowledgement: VisibleIssuanceAcknowledgement = {
    event: "visible_question_issued",
    namespace: scenarioNamespace,
    scenarioId: "webwork_delivery",
    schemaVersion: 1,
  };
  writeFileSync(path, JSON.stringify(acknowledgement), {
    encoding: "ascii",
    flag: "wx",
    mode: 0o600,
  });
}

function requiredPath(env: Readonly<Record<string, string | undefined>>, key: string): string {
  const value = env[key]?.trim();
  if (value === undefined || value === "") throw new Error(`webwork delivery requires ${key}`);
  return value;
}

function readPrivateAscii(path: string, label: string): string {
  const stat = lstatSync(path);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > maximumBytes) {
    throw new Error(`${label} is invalid`);
  }
  if (process.platform !== "win32" && (stat.mode & 0o777) !== 0o600) {
    throw new Error(`${label} is invalid`);
  }
  const contents = readFileSync(path, "utf8");
  if (!isAscii(contents)) throw new Error(`${label} is invalid`);
  return contents;
}

function parseRecord(contents: string): RecordValue {
  let value: unknown;
  try {
    value = JSON.parse(contents);
  } catch {
    throw new Error("WebWork Published Question fixture input is not valid JSON");
  }
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error("WebWork Published Question fixture input is invalid");
  }
  return value as RecordValue;
}

function keyList(value: RecordValue): string {
  return Object.keys(value).sort().join(",");
}

function isAscii(value: string): boolean {
  return [...value].every((character) => (character.codePointAt(0) ?? 0x80) <= 0x7f);
}

function requireCanonical(contents: string, value: WebworkPublishedQuestionFixtureInput): void {
  const canonical = JSON.stringify({
    questionId: value.questionId,
    scenarioId: value.scenarioId,
    schemaVersion: value.schemaVersion,
    title: value.title,
  });
  if (contents !== canonical) {
    throw new Error("WebWork Published Question fixture input must use canonical ASCII JSON");
  }
}
