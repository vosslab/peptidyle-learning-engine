import { DecodeError, decodeRecord } from "../../api/decoder";
import {
  FLAT_QUESTION_FORMAT,
  FLAT_QUESTION_RESPONSE_KIND,
  FLAT_QUESTION_VERSION,
  type FlatQuestionAttemptPolicy,
  type FlatQuestionChoice,
  type FlatQuestionFeedbackDisclosure,
  type FlatQuestionLicense,
  type FlatQuestionOutcomeFeedback,
  type FlatQuestionSourceV2,
  type FlatQuestionTaxonomyTerm,
  type FlatQuestionTimingPolicy,
} from "./flat_question_source";

const MAX_SOURCE_BYTES = 256 * 1024;
const MAX_CHOICES = 100;
const MAX_CHOICE_ID_BYTES = 64;
const MAX_TITLE_CHARS = 512;
const MAX_U32 = 4_294_967_295;
const MAX_PROMPT_CHARS = 65_536;
const MAX_CHOICE_TEXT_CHARS = 16_384;
const MAX_FEEDBACK_CHARS = 16_384;
const MAX_TAG_CHARS = 128;
const MAX_METADATA_CHARS = 256;
const FEEDBACK_DISCLOSURES = [
  "immediateFull",
  "immediateCorrectness",
  "deferred",
  "onRelease",
] as const;

function field(record: Record<string, unknown>, key: string, path: string): unknown {
  if (!(key in record)) throw new DecodeError(`${path}.${key}`, "present");
  return record[key];
}

function onlyFields(
  record: Record<string, unknown>,
  path: string,
  fields: ReadonlyArray<string>,
): void {
  for (const key of Object.keys(record)) {
    if (!fields.includes(key)) {
      throw new DecodeError(`${path}.${key}`, "a field allowed by PLE flat-question JSON v2");
    }
  }
}

function string(value: unknown, path: string): string {
  if (typeof value !== "string") throw new DecodeError(path, "a string");
  return value;
}

function boundedText(value: unknown, path: string, maximum: number): string {
  const decoded = string(value, path);
  if (decoded.trim().length === 0 || Array.from(decoded).length > maximum) {
    throw new DecodeError(path, `nonblank text no longer than ${maximum} characters`);
  }
  return decoded;
}

function integer(value: unknown, path: string, minimum: number, maximum: number): number {
  if (
    typeof value !== "number" ||
    !Number.isSafeInteger(value) ||
    value < minimum ||
    value > maximum
  ) {
    throw new DecodeError(path, `a safe integer from ${minimum} through ${maximum}`);
  }
  return value;
}

function finiteNonnegative(value: unknown, path: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
    throw new DecodeError(path, "a finite nonnegative number");
  }
  return value;
}

function decodeChoice(value: unknown, path: string): FlatQuestionChoice {
  const record = decodeRecord(value, path);
  onlyFields(record, path, ["id", "text", "feedback"]);
  const id = string(field(record, "id", path), `${path}.id`);
  if (
    new TextEncoder().encode(id).length > MAX_CHOICE_ID_BYTES ||
    !/^[a-z][a-z0-9_-]*$/u.test(id)
  ) {
    throw new DecodeError(`${path}.id`, "a lowercase semantic choice identifier");
  }
  const text = boundedText(field(record, "text", path), `${path}.text`, MAX_CHOICE_TEXT_CHARS);
  const feedbackValue = record.feedback;
  if (feedbackValue === undefined || feedbackValue === null) return { id, text, feedback: null };
  const feedback = boundedText(feedbackValue, `${path}.feedback`, MAX_FEEDBACK_CHARS);
  return { id, text, feedback };
}

function decodeOutcomeFeedback(value: unknown, path: string): FlatQuestionOutcomeFeedback {
  const record = decodeRecord(value, path);
  onlyFields(record, path, ["correct", "incorrect"]);
  const correct = record.correct;
  const incorrect = record.incorrect;
  const decodedCorrect =
    correct === undefined || correct === null
      ? null
      : boundedText(correct, `${path}.correct`, MAX_FEEDBACK_CHARS);
  const decodedIncorrect =
    incorrect === undefined || incorrect === null
      ? null
      : boundedText(incorrect, `${path}.incorrect`, MAX_FEEDBACK_CHARS);
  return { correct: decodedCorrect, incorrect: decodedIncorrect };
}

function decodeAttemptPolicy(value: unknown, path: string): FlatQuestionAttemptPolicy {
  const record = decodeRecord(value, path);
  onlyFields(record, path, ["maxAttempts", "feedback"]);
  const attemptsValue = field(record, "maxAttempts", path);
  const maxAttempts =
    attemptsValue === null ? null : integer(attemptsValue, `${path}.maxAttempts`, 1, MAX_U32);
  const feedback = string(field(record, "feedback", path), `${path}.feedback`);
  return { maxAttempts, feedback: decodeFeedbackDisclosure(feedback, `${path}.feedback`) };
}

function decodeFeedbackDisclosure(value: string, path: string): FlatQuestionFeedbackDisclosure {
  for (const candidate of FEEDBACK_DISCLOSURES) {
    if (candidate === value) return candidate;
  }
  throw new DecodeError(path, "a known feedback disclosure policy");
}

function decodeTimingPolicy(value: unknown, path: string): FlatQuestionTimingPolicy {
  const record = decodeRecord(value, path);
  const kind = string(field(record, "kind", path), `${path}.kind`);
  if (kind === "untimed") {
    onlyFields(record, path, ["kind"]);
    return { kind };
  }
  if (kind !== "perQuestion" && kind !== "perAttempt") {
    throw new DecodeError(`${path}.kind`, "a known timing policy");
  }
  onlyFields(record, path, ["kind", "seconds", "graceSeconds"]);
  const seconds = integer(field(record, "seconds", path), `${path}.seconds`, 1, MAX_U32);
  const graceSeconds = integer(
    field(record, "graceSeconds", path),
    `${path}.graceSeconds`,
    0,
    MAX_U32,
  );
  return { kind, seconds, graceSeconds };
}

function decodeTags(value: unknown, path: string): ReadonlyArray<string> {
  if (!Array.isArray(value)) throw new DecodeError(path, "an array");
  return value.map((entry, index) => boundedText(entry, `${path}[${index}]`, MAX_TAG_CHARS));
}

function decodeTaxonomy(value: unknown, path: string): ReadonlyArray<FlatQuestionTaxonomyTerm> {
  if (!Array.isArray(value)) throw new DecodeError(path, "an array");
  return value.map((entry, index) => {
    const entryPath = `${path}[${index}]`;
    const record = decodeRecord(entry, entryPath);
    onlyFields(record, entryPath, ["scheme", "code", "label"]);
    return {
      scheme: boundedText(
        field(record, "scheme", entryPath),
        `${entryPath}.scheme`,
        MAX_METADATA_CHARS,
      ),
      code: boundedText(field(record, "code", entryPath), `${entryPath}.code`, MAX_METADATA_CHARS),
      label: boundedText(
        field(record, "label", entryPath),
        `${entryPath}.label`,
        MAX_METADATA_CHARS,
      ),
    };
  });
}

function decodeLicense(value: unknown, path: string): FlatQuestionLicense {
  const record = decodeRecord(value, path);
  const kind = string(field(record, "kind", path), `${path}.kind`);
  if (kind === "other") {
    onlyFields(record, path, ["kind", "spdx"]);
    const spdx = boundedText(field(record, "spdx", path), `${path}.spdx`, MAX_METADATA_CHARS);
    return { kind, spdx };
  }
  if (!["allRightsReserved", "ccBy", "ccBySa", "ccByNc", "cc0"].includes(kind)) {
    throw new DecodeError(`${path}.kind`, "a known license kind");
  }
  onlyFields(record, path, ["kind"]);
  if (kind === "allRightsReserved") return { kind };
  if (kind === "ccBy") return { kind };
  if (kind === "ccBySa") return { kind };
  if (kind === "ccByNc") return { kind };
  return { kind: "cc0" };
}

/** Strictly decodes source returned by the protected authoring endpoint. */
export function decodeFlatQuestionSource(value: unknown, path = "source"): FlatQuestionSourceV2 {
  const record = decodeRecord(value, path);
  onlyFields(record, path, [
    "format",
    "version",
    "title",
    "prompt",
    "response",
    "feedback",
    "points",
    "attemptPolicy",
    "timingPolicy",
    "tags",
    "taxonomy",
    "license",
    "language",
  ]);
  if (field(record, "format", path) !== FLAT_QUESTION_FORMAT) {
    throw new DecodeError(`${path}.format`, `the literal ${FLAT_QUESTION_FORMAT}`);
  }
  if (field(record, "version", path) !== FLAT_QUESTION_VERSION) {
    throw new DecodeError(`${path}.version`, `the literal ${FLAT_QUESTION_VERSION}`);
  }
  const responsePath = `${path}.response`;
  const response = decodeRecord(field(record, "response", path), responsePath);
  onlyFields(response, responsePath, ["kind", "choices", "correctChoice"]);
  if (field(response, "kind", responsePath) !== FLAT_QUESTION_RESPONSE_KIND) {
    throw new DecodeError(`${responsePath}.kind`, `the literal ${FLAT_QUESTION_RESPONSE_KIND}`);
  }
  const choicesValue = field(response, "choices", responsePath);
  if (
    !Array.isArray(choicesValue) ||
    choicesValue.length < 2 ||
    choicesValue.length > MAX_CHOICES
  ) {
    throw new DecodeError(`${responsePath}.choices`, "an array of 2 to 100 choices");
  }
  const choices = choicesValue.map((choice, index) =>
    decodeChoice(choice, `${responsePath}.choices[${index}]`),
  );
  const identifiers = new Set<string>();
  for (const choice of choices) {
    if (identifiers.has(choice.id))
      throw new DecodeError(`${responsePath}.choices`, "unique choice identifiers");
    identifiers.add(choice.id);
  }
  const correctChoice = string(
    field(response, "correctChoice", responsePath),
    `${responsePath}.correctChoice`,
  );
  if (!identifiers.has(correctChoice)) {
    throw new DecodeError(`${responsePath}.correctChoice`, "an identifier of an available choice");
  }
  return {
    format: FLAT_QUESTION_FORMAT,
    version: FLAT_QUESTION_VERSION,
    title: boundedText(field(record, "title", path), `${path}.title`, MAX_TITLE_CHARS),
    prompt: boundedText(field(record, "prompt", path), `${path}.prompt`, MAX_PROMPT_CHARS),
    response: { kind: FLAT_QUESTION_RESPONSE_KIND, choices, correctChoice },
    feedback:
      record.feedback === undefined
        ? { correct: null, incorrect: null }
        : decodeOutcomeFeedback(record.feedback, `${path}.feedback`),
    points: finiteNonnegative(field(record, "points", path), `${path}.points`),
    attemptPolicy: decodeAttemptPolicy(
      field(record, "attemptPolicy", path),
      `${path}.attemptPolicy`,
    ),
    timingPolicy: decodeTimingPolicy(field(record, "timingPolicy", path), `${path}.timingPolicy`),
    tags: record.tags === undefined ? [] : decodeTags(record.tags, `${path}.tags`),
    taxonomy:
      record.taxonomy === undefined ? [] : decodeTaxonomy(record.taxonomy, `${path}.taxonomy`),
    license: decodeLicense(field(record, "license", path), `${path}.license`),
    language: boundedText(field(record, "language", path), `${path}.language`, MAX_METADATA_CHARS),
  };
}

/** Parses bounded JSON bytes from the protected source endpoint. */
export function parseFlatQuestionSource(text: string): FlatQuestionSourceV2 {
  if (new TextEncoder().encode(text).length > MAX_SOURCE_BYTES) {
    throw new DecodeError("source", `JSON no larger than ${MAX_SOURCE_BYTES} bytes`);
  }
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    throw new DecodeError("source", "valid JSON");
  }
  return decodeFlatQuestionSource(value);
}

/** Emits the Rust-compatible compact member order used for canonical source bytes. */
export function serializeFlatQuestionSource(source: FlatQuestionSourceV2): string {
  const valid = decodeFlatQuestionSource(source);
  const text = JSON.stringify(valid);
  if (new TextEncoder().encode(text).length > MAX_SOURCE_BYTES) {
    throw new DecodeError("source", `JSON no larger than ${MAX_SOURCE_BYTES} bytes`);
  }
  return text;
}
