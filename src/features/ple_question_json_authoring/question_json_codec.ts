import { DecodeError, decodeRecord } from "../../api/decoder";
import {
  PLE_QUESTION_JSON_FORMAT,
  PLE_QUESTION_JSON_FILL_IN_RESPONSE_KIND,
  PLE_QUESTION_JSON_HOTSPOT_RESPONSE_KIND,
  PLE_QUESTION_JSON_MATCHING_RESPONSE_KIND,
  PLE_QUESTION_JSON_MULTI_FILL_IN_RESPONSE_KIND,
  PLE_QUESTION_JSON_MULTIPLE_ANSWER_RESPONSE_KIND,
  PLE_QUESTION_JSON_NUMERIC_RESPONSE_KIND,
  PLE_QUESTION_JSON_ORDERING_RESPONSE_KIND,
  PLE_QUESTION_JSON_SINGLE_CHOICE_RESPONSE_KIND,
  PLE_QUESTION_JSON_SCHEMA_VERSION,
  createPleQuestionJsonMatchingChoice,
  createPleQuestionJsonMatchingPrompt,
  createPleQuestionJsonOrderingItem,
  type PleQuestionJsonAttemptLimit,
  type PleQuestionJsonBlank,
  type PleQuestionJsonChoice,
  type PleQuestionJsonHotspotRegion,
  type PleQuestionJsonHotspotSurface,
  type PleQuestionJsonMatchingChoice,
  type PleQuestionJsonMatchingPrompt,
  type PleQuestionJsonMatch,
  type PleQuestionJsonNumericResponseTolerance,
  type PleQuestionJsonOutcomeFeedback,
  type PleQuestionJsonDocument,
  type PleQuestionJsonClassification,
  type PleQuestionJsonAttemptTimeLimit,
  type PleQuestionJsonTextResponseMatchRule,
  type PleQuestionJsonOrderingItem,
} from "./question_json_source";

const MAX_SOURCE_BYTES = 256 * 1024;
const MAX_CHOICES = 100;
const MAX_CHOICE_ID_BYTES = 64;
const MAX_TITLE_CHARS = 512;
const MAX_QUESTION_DESCRIPTION_CHARS = 4_000;
const MAX_U32 = 4_294_967_295;
const MAX_PROMPT_CHARS = 65_536;
const MAX_CHOICE_TEXT_CHARS = 16_384;
const MAX_FEEDBACK_CHARS = 16_384;
const MAX_HINT_CHARS = 16_384;
const MAX_BLANKS = 50;
const MAX_TEXT_RESPONSE_CHARS = 16_384;
const MAX_NORMALIZED_COORDINATE = 10_000;
const MAX_TAG_CHARS = 128;
const MAX_METADATA_CHARS = 256;
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
      throw new DecodeError(
        `${path}.${key}`,
        "a field allowed by PLE Question JSON schema version 2",
      );
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

function decodeChoice(value: unknown, path: string): PleQuestionJsonChoice {
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

type PleQuestionJsonResponseMember = {
  readonly id: string;
  readonly text: string;
};

function decodeResponseMember(
  value: unknown,
  path: string,
  role: string,
): PleQuestionJsonResponseMember {
  const record = decodeRecord(value, path);
  onlyFields(record, path, ["id", "text"]);
  const id = string(field(record, "id", path), `${path}.id`);
  if (
    new TextEncoder().encode(id).length > MAX_CHOICE_ID_BYTES ||
    !/^[a-z][a-z0-9_-]*$/u.test(id)
  ) {
    throw new DecodeError(`${path}.id`, `a lowercase semantic ${role} identifier`);
  }
  const text = boundedText(field(record, "text", path), `${path}.text`, MAX_CHOICE_TEXT_CHARS);
  return { id, text };
}

function decodeResponseMembers<T extends PleQuestionJsonResponseMember>(
  value: unknown,
  path: string,
  label: string,
  decode: (member: unknown, memberPath: string) => T,
): ReadonlyArray<T> {
  if (!Array.isArray(value) || value.length < 2 || value.length > MAX_CHOICES) {
    throw new DecodeError(path, `an array of 2 to ${MAX_CHOICES} ${label}`);
  }
  const items = value.map((item, index) => decode(item, `${path}[${index}]`));
  const identifiers = new Set(items.map((item) => item.id));
  if (identifiers.size !== items.length) {
    throw new DecodeError(path, `unique ${label} identifiers`);
  }
  return items;
}

function decodeMatchingPrompt(value: unknown, path: string): PleQuestionJsonMatchingPrompt {
  const member = decodeResponseMember(value, path, "matching prompt");
  return createPleQuestionJsonMatchingPrompt(member.id, member.text);
}

function decodeMatchingPrompts(
  value: unknown,
  path: string,
): ReadonlyArray<PleQuestionJsonMatchingPrompt> {
  return decodeResponseMembers(value, path, "prompt", decodeMatchingPrompt);
}

function decodeMatchingChoice(value: unknown, path: string): PleQuestionJsonMatchingChoice {
  const member = decodeResponseMember(value, path, "matching choice");
  return createPleQuestionJsonMatchingChoice(member.id, member.text);
}

function decodeMatchingChoices(
  value: unknown,
  path: string,
): ReadonlyArray<PleQuestionJsonMatchingChoice> {
  return decodeResponseMembers(value, path, "choice", decodeMatchingChoice);
}

function decodeOrderingItem(value: unknown, path: string): PleQuestionJsonOrderingItem {
  const member = decodeResponseMember(value, path, "ordering item");
  return createPleQuestionJsonOrderingItem(member.id, member.text);
}

function decodeOrderingItems(
  value: unknown,
  path: string,
): ReadonlyArray<PleQuestionJsonOrderingItem> {
  return decodeResponseMembers(value, path, "ordering item", decodeOrderingItem);
}

function decodeTextResponseMatchRule(
  value: unknown,
  path: string,
): PleQuestionJsonTextResponseMatchRule {
  const mode = string(value, path);
  if (mode === "exact" || mode === "caseInsensitive" || mode === "normalized") return mode;
  throw new DecodeError(path, "a known text match mode");
}

function decodeAnswers(value: unknown, path: string, maxLength: number): ReadonlyArray<string> {
  if (!Array.isArray(value) || value.length === 0) {
    throw new DecodeError(path, "a nonempty array of accepted answers");
  }
  const answers = value.map((answer, index) =>
    boundedText(answer, `${path}[${index}]`, MAX_FEEDBACK_CHARS),
  );
  if (new Set(answers).size !== answers.length) {
    throw new DecodeError(path, "unique accepted answers");
  }
  if (maxLength < 1 || maxLength > MAX_TEXT_RESPONSE_CHARS) {
    throw new DecodeError(
      `${path}.maxLength`,
      `an integer from 1 through ${MAX_TEXT_RESPONSE_CHARS}`,
    );
  }
  return answers;
}

function decodeTextMaxLength(value: unknown, path: string): number {
  return integer(value, path, 1, MAX_TEXT_RESPONSE_CHARS);
}

function decodeNumericResponseTolerance(
  value: unknown,
  path: string,
): PleQuestionJsonNumericResponseTolerance {
  const record = decodeRecord(value, path);
  const kind = string(field(record, "kind", path), `${path}.kind`);
  if (kind === "exact") {
    onlyFields(record, path, ["kind"]);
    return { kind };
  }
  if (kind === "absolute" || kind === "relative") {
    const fieldName = kind === "absolute" ? "epsilon" : "fraction";
    onlyFields(record, path, ["kind", fieldName]);
    const amount = finiteNonnegative(field(record, fieldName, path), `${path}.${fieldName}`);
    return kind === "absolute" ? { kind, epsilon: amount } : { kind, fraction: amount };
  }
  if (kind === "significantFigures") {
    onlyFields(record, path, ["kind", "digits"]);
    return { kind, digits: integer(field(record, "digits", path), `${path}.digits`, 1, 255) };
  }
  throw new DecodeError(`${path}.kind`, "a known numeric tolerance");
}

function decodeBlank(value: unknown, path: string): PleQuestionJsonBlank {
  const record = decodeRecord(value, path);
  onlyFields(record, path, ["id", "label", "answers", "matchMode", "maxLength"]);
  const id = string(field(record, "id", path), `${path}.id`);
  if (
    new TextEncoder().encode(id).length > MAX_CHOICE_ID_BYTES ||
    !/^[a-z][a-z0-9_-]*$/u.test(id)
  ) {
    throw new DecodeError(`${path}.id`, "a lowercase semantic blank identifier");
  }
  const maxLength = decodeTextMaxLength(field(record, "maxLength", path), `${path}.maxLength`);
  return {
    id,
    label: boundedText(field(record, "label", path), `${path}.label`, MAX_CHOICE_TEXT_CHARS),
    answers: decodeAnswers(field(record, "answers", path), `${path}.answers`, maxLength),
    matchMode: decodeTextResponseMatchRule(field(record, "matchMode", path), `${path}.matchMode`),
    maxLength,
  };
}

function decodeChoiceResponse(
  value: unknown,
  path: string,
  multiple: boolean,
): Extract<
  PleQuestionJsonDocument["response"],
  { readonly kind: "singleChoice" | "multipleAnswer" }
> {
  const record = decodeRecord(value, path);
  const correctField = multiple ? "correctChoices" : "correctChoice";
  onlyFields(record, path, ["kind", "choices", correctField]);
  const expectedKind = multiple
    ? PLE_QUESTION_JSON_MULTIPLE_ANSWER_RESPONSE_KIND
    : PLE_QUESTION_JSON_SINGLE_CHOICE_RESPONSE_KIND;
  if (field(record, "kind", path) !== expectedKind) {
    throw new DecodeError(`${path}.kind`, `the literal ${expectedKind}`);
  }
  const choicesValue = field(record, "choices", path);
  if (
    !Array.isArray(choicesValue) ||
    choicesValue.length < 2 ||
    choicesValue.length > MAX_CHOICES
  ) {
    throw new DecodeError(`${path}.choices`, "an array of 2 to 100 choices");
  }
  const choices = choicesValue.map((choice, index) =>
    decodeChoice(choice, `${path}.choices[${index}]`),
  );
  const identifiers = new Set(choices.map((choice) => choice.id));
  if (identifiers.size !== choices.length) {
    throw new DecodeError(`${path}.choices`, "unique choice identifiers");
  }
  if (!multiple) {
    const correctChoice = string(field(record, correctField, path), `${path}.${correctField}`);
    if (!identifiers.has(correctChoice)) {
      throw new DecodeError(`${path}.${correctField}`, "an identifier of an available choice");
    }
    return { kind: PLE_QUESTION_JSON_SINGLE_CHOICE_RESPONSE_KIND, choices, correctChoice };
  }
  const correctValue = field(record, correctField, path);
  if (!Array.isArray(correctValue) || correctValue.length === 0) {
    throw new DecodeError(
      `${path}.${correctField}`,
      "a nonempty array of available choice identifiers",
    );
  }
  const correctChoices = correctValue.map((choice, index) =>
    string(choice, `${path}.${correctField}[${index}]`),
  );
  if (
    new Set(correctChoices).size !== correctChoices.length ||
    !correctChoices.every((id) => identifiers.has(id))
  ) {
    throw new DecodeError(`${path}.${correctField}`, "unique identifiers of available choices");
  }
  return { kind: PLE_QUESTION_JSON_MULTIPLE_ANSWER_RESPONSE_KIND, choices, correctChoices };
}

function decodeHotspotSurface(value: unknown, path: string): PleQuestionJsonHotspotSurface {
  const record = decodeRecord(value, path);
  onlyFields(record, path, ["asset", "checksum", "description"]);
  const asset = string(field(record, "asset", path), `${path}.asset`);
  if (!/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/iu.test(asset)) {
    throw new DecodeError(`${path}.asset`, "a UUID");
  }
  const checksum = string(field(record, "checksum", path), `${path}.checksum`);
  if (!/^[0-9a-f]{64}$/u.test(checksum)) {
    throw new DecodeError(`${path}.checksum`, "a lowercase SHA-256 hexadecimal digest");
  }
  return {
    asset,
    checksum,
    description: boundedText(
      field(record, "description", path),
      `${path}.description`,
      MAX_CHOICE_TEXT_CHARS,
    ),
  };
}

function decodeHotspotCoordinate(value: unknown, path: string): number {
  return integer(value, path, 0, MAX_NORMALIZED_COORDINATE);
}

function decodeHotspotRegion(value: unknown, path: string): PleQuestionJsonHotspotRegion {
  const record = decodeRecord(value, path);
  onlyFields(record, path, ["id", "label", "x", "y", "width", "height"]);
  const id = string(field(record, "id", path), `${path}.id`);
  if (
    new TextEncoder().encode(id).length > MAX_CHOICE_ID_BYTES ||
    !/^[a-z][a-z0-9_-]*$/u.test(id)
  ) {
    throw new DecodeError(`${path}.id`, "a lowercase semantic region identifier");
  }
  const x = decodeHotspotCoordinate(field(record, "x", path), `${path}.x`);
  const y = decodeHotspotCoordinate(field(record, "y", path), `${path}.y`);
  const width = decodeHotspotCoordinate(field(record, "width", path), `${path}.width`);
  const height = decodeHotspotCoordinate(field(record, "height", path), `${path}.height`);
  if (
    width === 0 ||
    height === 0 ||
    x + width > MAX_NORMALIZED_COORDINATE ||
    y + height > MAX_NORMALIZED_COORDINATE
  ) {
    throw new DecodeError(path, "a nonempty normalized rectangle inside the hotspot surface");
  }
  return {
    id,
    label: boundedText(field(record, "label", path), `${path}.label`, MAX_CHOICE_TEXT_CHARS),
    x,
    y,
    width,
    height,
  };
}

function hotspotRegionsOverlap(
  left: PleQuestionJsonHotspotRegion,
  right: PleQuestionJsonHotspotRegion,
): boolean {
  return (
    left.x <= right.x + right.width &&
    right.x <= left.x + left.width &&
    left.y <= right.y + right.height &&
    right.y <= left.y + left.height
  );
}

function decodeHotspotResponse(
  value: unknown,
  path: string,
): Extract<PleQuestionJsonDocument["response"], { readonly kind: "hotspot" }> {
  const record = decodeRecord(value, path);
  onlyFields(record, path, ["kind", "surface", "regions", "correctRegions"]);
  if (field(record, "kind", path) !== PLE_QUESTION_JSON_HOTSPOT_RESPONSE_KIND) {
    throw new DecodeError(`${path}.kind`, `the literal ${PLE_QUESTION_JSON_HOTSPOT_RESPONSE_KIND}`);
  }
  const regionsValue = field(record, "regions", path);
  if (
    !Array.isArray(regionsValue) ||
    regionsValue.length === 0 ||
    regionsValue.length > MAX_CHOICES
  ) {
    throw new DecodeError(`${path}.regions`, "an array of 1 to 100 hotspot regions");
  }
  const regions = regionsValue.map((region, index) =>
    decodeHotspotRegion(region, `${path}.regions[${index}]`),
  );
  const regionIds = new Set(regions.map((region) => region.id));
  if (regionIds.size !== regions.length) {
    throw new DecodeError(`${path}.regions`, "unique hotspot region identifiers");
  }
  for (let index = 0; index < regions.length; index += 1) {
    const left = regions[index];
    if (
      left !== undefined &&
      regions.slice(index + 1).some((right) => hotspotRegionsOverlap(left, right))
    ) {
      throw new DecodeError(`${path}.regions`, "nonoverlapping hotspot regions");
    }
  }
  const correctValue = field(record, "correctRegions", path);
  if (!Array.isArray(correctValue) || correctValue.length === 0) {
    throw new DecodeError(
      `${path}.correctRegions`,
      "a nonempty array of hotspot region identifiers",
    );
  }
  const correctRegions = correctValue.map((region, index) =>
    string(region, `${path}.correctRegions[${index}]`),
  );
  if (
    new Set(correctRegions).size !== correctRegions.length ||
    !correctRegions.every((id) => regionIds.has(id))
  ) {
    throw new DecodeError(`${path}.correctRegions`, "unique available hotspot region identifiers");
  }
  return {
    kind: PLE_QUESTION_JSON_HOTSPOT_RESPONSE_KIND,
    surface: decodeHotspotSurface(field(record, "surface", path), `${path}.surface`),
    regions,
    correctRegions,
  };
}

function decodeMatchingResponse(
  value: unknown,
  path: string,
): Extract<PleQuestionJsonDocument["response"], { readonly kind: "matching" }> {
  const record = decodeRecord(value, path);
  onlyFields(record, path, ["kind", "prompts", "choices", "matches"]);
  if (field(record, "kind", path) !== PLE_QUESTION_JSON_MATCHING_RESPONSE_KIND) {
    throw new DecodeError(
      `${path}.kind`,
      `the literal ${PLE_QUESTION_JSON_MATCHING_RESPONSE_KIND}`,
    );
  }
  const prompts = decodeMatchingPrompts(field(record, "prompts", path), `${path}.prompts`);
  const choices = decodeMatchingChoices(field(record, "choices", path), `${path}.choices`);
  if (prompts.length > choices.length) {
    throw new DecodeError(`${path}.choices`, "at least as many choices as prompts");
  }
  const matchesValue = field(record, "matches", path);
  if (!Array.isArray(matchesValue) || matchesValue.length !== prompts.length) {
    throw new DecodeError(`${path}.matches`, "one pairing for each prompt");
  }
  const matches: PleQuestionJsonMatch[] = matchesValue.map((pair, index) => {
    const pairPath = `${path}.matches[${index}]`;
    const pairRecord = decodeRecord(pair, pairPath);
    onlyFields(pairRecord, pairPath, ["prompt", "choice"]);
    return {
      prompt: string(field(pairRecord, "prompt", pairPath), `${pairPath}.prompt`),
      choice: string(field(pairRecord, "choice", pairPath), `${pairPath}.choice`),
    };
  });
  const promptIds = new Set(prompts.map((prompt) => prompt.id));
  const choiceIds = new Set(choices.map((choice) => choice.id));
  const matchedPrompts = new Set(matches.map((pair) => pair.prompt));
  const matchedChoices = new Set(matches.map((pair) => pair.choice));
  if (
    matchedPrompts.size !== matches.length ||
    matchedChoices.size !== matches.length ||
    matchedPrompts.size !== promptIds.size ||
    ![...matchedPrompts].every((id) => promptIds.has(id)) ||
    ![...matchedChoices].every((id) => choiceIds.has(id))
  ) {
    throw new DecodeError(
      `${path}.matches`,
      "one unique available choice paired with every available prompt",
    );
  }
  return { kind: PLE_QUESTION_JSON_MATCHING_RESPONSE_KIND, prompts, choices, matches } as const;
}

function decodeOutcomeFeedback(value: unknown, path: string): PleQuestionJsonOutcomeFeedback {
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

function decodeQuestionHint(value: unknown, path: string): string | null {
  return value === undefined || value === null ? null : boundedText(value, path, MAX_HINT_CHARS);
}

function decodeQuestionAttemptLimit(value: unknown, path: string): PleQuestionJsonAttemptLimit {
  const record = decodeRecord(value, path);
  onlyFields(record, path, ["maxAttempts"]);
  const attemptsValue = field(record, "maxAttempts", path);
  return {
    maxAttempts:
      attemptsValue === null ? null : integer(attemptsValue, `${path}.maxAttempts`, 1, MAX_U32),
  };
}

function decodeQuestionAttemptTimeLimit(
  value: unknown,
  path: string,
): PleQuestionJsonAttemptTimeLimit {
  const record = decodeRecord(value, path);
  const kind = string(field(record, "kind", path), `${path}.kind`);
  if (kind === "unlimited") {
    onlyFields(record, path, ["kind"]);
    return { kind };
  }
  if (kind !== "limited" && kind !== "limited") {
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

function decodeClassifications(
  value: unknown,
  path: string,
): ReadonlyArray<PleQuestionJsonClassification> {
  if (!Array.isArray(value)) throw new DecodeError(path, "an array");
  return value.map((entry, index) => {
    const entryPath = `${path}[${index}]`;
    const record = decodeRecord(entry, entryPath);
    onlyFields(record, entryPath, ["system", "code", "name"]);
    return {
      system: boundedText(
        field(record, "system", entryPath),
        `${entryPath}.system`,
        MAX_METADATA_CHARS,
      ),
      code: boundedText(field(record, "code", entryPath), `${entryPath}.code`, MAX_METADATA_CHARS),
      name: boundedText(field(record, "name", entryPath), `${entryPath}.name`, MAX_METADATA_CHARS),
    };
  });
}

function decodeQuestionLicense(
  value: unknown,
  path: string,
): PleQuestionJsonDocument["questionLicense"] {
  if (value === null) return null;
  if (value === "CC0-1.0" || value === "CC-BY-4.0" || value === "CC-BY-SA-4.0") return value;
  throw new DecodeError(path, "an exact Question License or null");
}

function decodeQuestionCitation(
  value: unknown,
  path: string,
): PleQuestionJsonDocument["questionCitation"] {
  if (value === null) return null;
  const record = decodeRecord(value, path);
  onlyFields(record, path, ["citationUrl", "citationText"]);
  const citationUrl =
    field(record, "citationUrl", path) === null
      ? null
      : boundedText(field(record, "citationUrl", path), `${path}.citationUrl`, 2048);
  const citationText =
    field(record, "citationText", path) === null
      ? null
      : boundedText(field(record, "citationText", path), `${path}.citationText`, 4000);
  if (citationUrl === null && citationText === null) {
    throw new DecodeError(path, "a Citation URL or NLM-style Citation Text");
  }
  return { citationUrl, citationText };
}

/** Strictly decodes source returned by the protected authoring endpoint. */
export function decodePleQuestionJsonSource(
  value: unknown,
  path = "source",
): PleQuestionJsonDocument {
  const record = decodeRecord(value, path);
  onlyFields(record, path, [
    "format",
    "version",
    "title",
    "questionDescription",
    "prompt",
    "response",
    "questionHint",
    "feedback",
    "points",
    "questionAttemptLimit",
    "questionAttemptTimeLimit",
    "tags",
    "classifications",
    "questionLicense",
    "questionCitation",
    "language",
  ]);
  if (field(record, "format", path) !== PLE_QUESTION_JSON_FORMAT) {
    throw new DecodeError(`${path}.format`, `the literal ${PLE_QUESTION_JSON_FORMAT}`);
  }
  if (field(record, "version", path) !== PLE_QUESTION_JSON_SCHEMA_VERSION) {
    throw new DecodeError(`${path}.version`, `the literal ${PLE_QUESTION_JSON_SCHEMA_VERSION}`);
  }
  const responsePath = `${path}.response`;
  const responseValue = field(record, "response", path);
  const responseRecord = decodeRecord(responseValue, responsePath);
  const responseKind = field(responseRecord, "kind", responsePath);
  let response: PleQuestionJsonDocument["response"];
  if (responseKind === PLE_QUESTION_JSON_SINGLE_CHOICE_RESPONSE_KIND) {
    response = decodeChoiceResponse(responseValue, responsePath, false);
  } else if (responseKind === PLE_QUESTION_JSON_MULTIPLE_ANSWER_RESPONSE_KIND) {
    response = decodeChoiceResponse(responseValue, responsePath, true);
  } else if (responseKind === PLE_QUESTION_JSON_MATCHING_RESPONSE_KIND) {
    response = decodeMatchingResponse(responseValue, responsePath);
  } else if (responseKind === PLE_QUESTION_JSON_FILL_IN_RESPONSE_KIND) {
    onlyFields(responseRecord, responsePath, ["kind", "answers", "matchMode", "maxLength"]);
    const maxLength = decodeTextMaxLength(
      field(responseRecord, "maxLength", responsePath),
      `${responsePath}.maxLength`,
    );
    response = {
      kind: PLE_QUESTION_JSON_FILL_IN_RESPONSE_KIND,
      answers: decodeAnswers(
        field(responseRecord, "answers", responsePath),
        `${responsePath}.answers`,
        maxLength,
      ),
      matchMode: decodeTextResponseMatchRule(
        field(responseRecord, "matchMode", responsePath),
        `${responsePath}.matchMode`,
      ),
      maxLength,
    };
  } else if (responseKind === PLE_QUESTION_JSON_MULTI_FILL_IN_RESPONSE_KIND) {
    onlyFields(responseRecord, responsePath, ["kind", "blanks"]);
    const blanksValue = field(responseRecord, "blanks", responsePath);
    if (
      !Array.isArray(blanksValue) ||
      blanksValue.length === 0 ||
      blanksValue.length > MAX_BLANKS
    ) {
      throw new DecodeError(`${responsePath}.blanks`, `an array of 1 to ${MAX_BLANKS} blanks`);
    }
    const blanks = blanksValue.map((blank, index) =>
      decodeBlank(blank, `${responsePath}.blanks[${index}]`),
    );
    if (new Set(blanks.map((blank) => blank.id)).size !== blanks.length) {
      throw new DecodeError(`${responsePath}.blanks`, "unique blank identifiers");
    }
    response = { kind: PLE_QUESTION_JSON_MULTI_FILL_IN_RESPONSE_KIND, blanks };
  } else if (responseKind === PLE_QUESTION_JSON_NUMERIC_RESPONSE_KIND) {
    onlyFields(responseRecord, responsePath, ["kind", "answer", "tolerance", "unit"]);
    const answer = field(responseRecord, "answer", responsePath);
    if (typeof answer !== "number" || !Number.isFinite(answer)) {
      throw new DecodeError(`${responsePath}.answer`, "a finite number");
    }
    const unitValue = responseRecord.unit;
    const unit =
      unitValue === undefined || unitValue === null
        ? null
        : boundedText(unitValue, `${responsePath}.unit`, MAX_METADATA_CHARS);
    response = {
      kind: PLE_QUESTION_JSON_NUMERIC_RESPONSE_KIND,
      answer,
      tolerance: decodeNumericResponseTolerance(
        field(responseRecord, "tolerance", responsePath),
        `${responsePath}.tolerance`,
      ),
      unit,
    };
  } else if (responseKind === PLE_QUESTION_JSON_ORDERING_RESPONSE_KIND) {
    onlyFields(responseRecord, responsePath, ["kind", "items", "correctOrder"]);
    const items = decodeOrderingItems(
      field(responseRecord, "items", responsePath),
      `${responsePath}.items`,
    );
    if (items.length < 3)
      throw new DecodeError(`${responsePath}.items`, "an array of 3 to 100 ordering items");
    const correctValue = field(responseRecord, "correctOrder", responsePath);
    if (!Array.isArray(correctValue) || correctValue.length !== items.length) {
      throw new DecodeError(
        `${responsePath}.correctOrder`,
        "one identifier for every ordering item",
      );
    }
    const correctOrder = correctValue.map((item, index) =>
      string(item, `${responsePath}.correctOrder[${index}]`),
    );
    const ids = new Set(items.map((item) => item.id));
    if (
      new Set(correctOrder).size !== correctOrder.length ||
      !correctOrder.every((id) => ids.has(id))
    ) {
      throw new DecodeError(`${responsePath}.correctOrder`, "every ordering item exactly once");
    }
    response = { kind: PLE_QUESTION_JSON_ORDERING_RESPONSE_KIND, items, correctOrder };
  } else if (responseKind === PLE_QUESTION_JSON_HOTSPOT_RESPONSE_KIND) {
    response = decodeHotspotResponse(responseValue, responsePath);
  } else {
    throw new DecodeError(`${responsePath}.kind`, "a supported PLE Question JSON response kind");
  }
  return {
    format: PLE_QUESTION_JSON_FORMAT,
    version: PLE_QUESTION_JSON_SCHEMA_VERSION,
    title: boundedText(field(record, "title", path), `${path}.title`, MAX_TITLE_CHARS),
    questionDescription: boundedText(
      field(record, "questionDescription", path),
      `${path}.questionDescription`,
      MAX_QUESTION_DESCRIPTION_CHARS,
    ),
    prompt: boundedText(field(record, "prompt", path), `${path}.prompt`, MAX_PROMPT_CHARS),
    response,
    questionHint: decodeQuestionHint(record.questionHint, `${path}.questionHint`),
    feedback:
      record.feedback === undefined
        ? { correct: null, incorrect: null }
        : decodeOutcomeFeedback(record.feedback, `${path}.feedback`),
    points: finiteNonnegative(field(record, "points", path), `${path}.points`),
    questionAttemptLimit: decodeQuestionAttemptLimit(
      field(record, "questionAttemptLimit", path),
      `${path}.questionAttemptLimit`,
    ),
    questionAttemptTimeLimit: decodeQuestionAttemptTimeLimit(
      field(record, "questionAttemptTimeLimit", path),
      `${path}.questionAttemptTimeLimit`,
    ),
    tags: record.tags === undefined ? [] : decodeTags(record.tags, `${path}.tags`),
    classifications:
      record.classifications === undefined
        ? []
        : decodeClassifications(record.classifications, `${path}.classifications`),
    questionLicense: decodeQuestionLicense(
      field(record, "questionLicense", path),
      `${path}.questionLicense`,
    ),
    questionCitation: decodeQuestionCitation(
      field(record, "questionCitation", path),
      `${path}.questionCitation`,
    ),
    language: boundedText(field(record, "language", path), `${path}.language`, MAX_METADATA_CHARS),
  };
}

/** Parses bounded JSON bytes from the protected source endpoint. */
export function parsePleQuestionJsonSource(text: string): PleQuestionJsonDocument {
  if (new TextEncoder().encode(text).length > MAX_SOURCE_BYTES) {
    throw new DecodeError("source", `JSON no larger than ${MAX_SOURCE_BYTES} bytes`);
  }
  let value: unknown;
  try {
    value = JSON.parse(text);
  } catch {
    throw new DecodeError("source", "valid JSON");
  }
  return decodePleQuestionJsonSource(value);
}

/** Emits the Rust-compatible compact member order used for canonical source bytes. */
export function serializePleQuestionJsonSource(source: PleQuestionJsonDocument): string {
  const valid = decodePleQuestionJsonSource(source);
  const text = JSON.stringify(valid);
  if (new TextEncoder().encode(text).length > MAX_SOURCE_BYTES) {
    throw new DecodeError("source", `JSON no larger than ${MAX_SOURCE_BYTES} bytes`);
  }
  return text;
}
