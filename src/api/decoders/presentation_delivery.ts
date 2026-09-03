// Strict Student Question Attempt View decoding and key-free Question Response Control translation.

import type { QuestionAssetReference } from "../../../generated/api/QuestionAssetReference";
import type { QuestionPresentation } from "../../../generated/api/QuestionPresentation";
import type { PresentedMatchingChoice } from "../../../generated/api/PresentedMatchingChoice";
import type { PresentedMatchingPrompt } from "../../../generated/api/PresentedMatchingPrompt";
import type { PresentedOrderingItem } from "../../../generated/api/PresentedOrderingItem";
import type { PresentedQuestionChoice } from "../../../generated/api/PresentedQuestionChoice";
import type { PresentedTextEntrySlot } from "../../../generated/api/PresentedTextEntrySlot";
import type { PresentedHotspotRegion } from "../../../generated/api/PresentedHotspotRegion";
import type { QuestionPresentationResponseFormat } from "../../../generated/api/QuestionPresentationResponseFormat";
import {
  DecodeError,
  decodeBoolean,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
} from "../decoder";
import {
  decodeBoundedArray,
  decodeQuestionTitle,
  decodeIdentifier,
  decodeQuestionRevisionReference,
  decodeSha256,
  field,
  kind,
  requireOnlyFields,
} from "./shared";
import { decodeQuestionContentBlock } from "./question_response_format";

const MAX_PRESENTED_ITEMS = 32;
const PRESENTATION_RESPONSE_ITEM_REFERENCE = /^[0-9a-f]{4}$/u;
const PRESENTATION_NONCE = /^[0-9a-f]{32}$/u;
type PresentedResponseItemFields = Pick<PresentedQuestionChoice, "id" | "body">;

function presentationResponseItemReference(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!PRESENTATION_RESPONSE_ITEM_REFERENCE.test(decoded)) {
    throw new DecodeError(path, "a four-character lowercase Presentation Response Item Reference");
  }
  return decoded;
}

function presentedResponseItemFields(value: unknown, path: string): PresentedResponseItemFields {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["id", "body"]);
  return {
    id: presentationResponseItemReference(field(record, "id", path), `${path}.id`),
    body: decodeBoundedArray(field(record, "body", path), `${path}.body`, 32, (block, blockPath) =>
      decodeQuestionContentBlock(block, blockPath, true),
    ),
  };
}

function presentedItems<T extends { id: string }>(
  value: unknown,
  path: string,
  decodeItem: (item: unknown, itemPath: string) => T,
): T[] {
  const choices = decodeBoundedArray(value, path, MAX_PRESENTED_ITEMS, decodeItem);
  if (choices.length < 2) throw new DecodeError(path, "at least two presented choices");
  const ids = choices.map((choice) => choice.id);
  if (new Set(ids).size !== ids.length) {
    throw new DecodeError(path, "presented choices with unique IDs");
  }
  return choices;
}

function presentedQuestionChoice(value: unknown, path: string): PresentedQuestionChoice {
  return presentedResponseItemFields(value, path);
}

function presentedMatchingPrompt(value: unknown, path: string): PresentedMatchingPrompt {
  return presentedResponseItemFields(value, path);
}

function presentedMatchingChoice(value: unknown, path: string): PresentedMatchingChoice {
  return presentedResponseItemFields(value, path);
}

function presentedOrderingItem(value: unknown, path: string): PresentedOrderingItem {
  return presentedResponseItemFields(value, path);
}

function presentedTextEntrySlot(value: unknown, path: string): PresentedTextEntrySlot {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["id", "label", "maxCharacters"]);
  return {
    id: presentationResponseItemReference(field(record, "id", path), `${path}.id`),
    label: decodeBoundedArray(
      field(record, "label", path),
      `${path}.label`,
      32,
      (block, blockPath) => decodeQuestionContentBlock(block, blockPath, true),
    ),
    maxCharacters: decodePositiveInteger(
      field(record, "maxCharacters", path),
      `${path}.maxCharacters`,
    ),
  };
}

function normalizedCoordinate(value: unknown, path: string): number {
  const decoded = decodeNonnegativeInteger(value, path);
  if (decoded > 10_000) throw new DecodeError(path, "an integer from 0 through 10000");
  return decoded;
}

function presentedHotspotRegion(value: unknown, path: string): PresentedHotspotRegion {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["id", "label", "x", "y", "width", "height"]);
  const x = normalizedCoordinate(field(record, "x", path), `${path}.x`);
  const y = normalizedCoordinate(field(record, "y", path), `${path}.y`);
  const width = decodePositiveInteger(field(record, "width", path), `${path}.width`);
  const height = decodePositiveInteger(field(record, "height", path), `${path}.height`);
  if (x + width > 10_000 || y + height > 10_000) {
    throw new DecodeError(path, "a rectangle within the normalized 10000 by 10000 surface");
  }
  return {
    id: presentationResponseItemReference(field(record, "id", path), `${path}.id`),
    label: decodeBoundedArray(
      field(record, "label", path),
      `${path}.label`,
      32,
      (block, blockPath) => decodeQuestionContentBlock(block, blockPath, true),
    ),
    x,
    y,
    width,
    height,
  };
}

function questionAssetReference(value: unknown, path: string): QuestionAssetReference {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["asset", "checksum"]);
  return {
    asset: decodeIdentifier(field(record, "asset", path), `${path}.asset`),
    checksum: decodeSha256(field(record, "checksum", path), `${path}.checksum`),
  };
}

function bounds(record: Record<string, unknown>, path: string, count: number): [number, number] {
  const minimum = decodeNonnegativeInteger(field(record, "minimum", path), `${path}.minimum`);
  const maximum = decodeNonnegativeInteger(field(record, "maximum", path), `${path}.maximum`);
  if (minimum > maximum || maximum > count) {
    throw new DecodeError(path, "selection bounds within the presented items");
  }
  return [minimum, maximum];
}

function issuedQuestionResponseFormat(
  value: unknown,
  path: string,
): QuestionPresentationResponseFormat {
  const record = decodeRecord(value, path);
  switch (kind(record, path)) {
    case "imathasQuestionBackend":
      requireOnlyFields(record, path, ["kind"]);
      return { kind: "imathasQuestionBackend" };
    case "singleChoice": {
      requireOnlyFields(record, path, ["kind", "choices"]);
      return {
        kind: "singleChoice",
        choices: presentedItems(
          field(record, "choices", path),
          `${path}.choices`,
          presentedQuestionChoice,
        ),
      };
    }
    case "multipleAnswer": {
      requireOnlyFields(record, path, ["kind", "choices", "minimum", "maximum"]);
      const choices = presentedItems(
        field(record, "choices", path),
        `${path}.choices`,
        presentedQuestionChoice,
      );
      const [minimum, maximum] = bounds(record, path, choices.length);
      return { kind: "multipleAnswer", choices, minimum, maximum };
    }
    case "fillIn":
      requireOnlyFields(record, path, ["kind", "maxCharacters"]);
      return {
        kind: "fillIn",
        maxCharacters: decodePositiveInteger(
          field(record, "maxCharacters", path),
          `${path}.maxCharacters`,
        ),
      };
    case "multiFillIn": {
      requireOnlyFields(record, path, ["kind", "blanks"]);
      const blanks = decodeBoundedArray(
        field(record, "blanks", path),
        `${path}.blanks`,
        MAX_PRESENTED_ITEMS,
        presentedTextEntrySlot,
      );
      if (blanks.length === 0 || new Set(blanks.map((blank) => blank.id)).size !== blanks.length) {
        throw new DecodeError(`${path}.blanks`, "one or more blanks with unique IDs");
      }
      return { kind: "multiFillIn", blanks };
    }
    case "numerical":
      requireOnlyFields(record, path, ["kind", "maxCharacters", "displayedUnit"]);
      return {
        kind: "numerical",
        maxCharacters: decodePositiveInteger(
          field(record, "maxCharacters", path),
          `${path}.maxCharacters`,
        ),
        displayedUnit: decodeNullable(
          field(record, "displayedUnit", path),
          `${path}.displayedUnit`,
          decodeString,
        ),
      };
    case "matching": {
      requireOnlyFields(record, path, ["kind", "prompts", "choices", "reuseChoices"]);
      const prompts = presentedItems(
        field(record, "prompts", path),
        `${path}.prompts`,
        presentedMatchingPrompt,
      );
      const choices = presentedItems(
        field(record, "choices", path),
        `${path}.choices`,
        presentedMatchingChoice,
      );
      const reuseChoices = decodeBoolean(
        field(record, "reuseChoices", path),
        `${path}.reuseChoices`,
      );
      if (!reuseChoices && prompts.length > choices.length) {
        throw new DecodeError(path, "enough matching choices for every prompt");
      }
      if (
        new Set([...prompts, ...choices].map((choice) => choice.id)).size !==
        prompts.length + choices.length
      ) {
        throw new DecodeError(path, "globally unique matching presentation IDs");
      }
      return { kind: "matching", prompts, choices, reuseChoices };
    }
    case "ordering": {
      requireOnlyFields(record, path, ["kind", "items"]);
      return {
        kind: "ordering",
        items: presentedItems(field(record, "items", path), `${path}.items`, presentedOrderingItem),
      };
    }
    case "hotspot": {
      requireOnlyFields(record, path, ["kind", "surface", "minimum", "maximum"]);
      const surfacePath = `${path}.surface`;
      const surfaceRecord = decodeRecord(field(record, "surface", path), surfacePath);
      requireOnlyFields(surfaceRecord, surfacePath, ["id", "asset", "description", "regions"]);
      const regions = decodeBoundedArray(
        field(surfaceRecord, "regions", surfacePath),
        `${surfacePath}.regions`,
        MAX_PRESENTED_ITEMS,
        presentedHotspotRegion,
      );
      if (regions.length === 0) {
        throw new DecodeError(`${surfacePath}.regions`, "one or more regions");
      }
      const [minimum, maximum] = bounds(record, path, regions.length);
      return {
        kind: "hotspot",
        surface: {
          id: presentationResponseItemReference(
            field(surfaceRecord, "id", surfacePath),
            `${surfacePath}.id`,
          ),
          asset: questionAssetReference(
            field(surfaceRecord, "asset", surfacePath),
            `${surfacePath}.asset`,
          ),
          description: decodeNonemptyString(
            field(surfaceRecord, "description", surfacePath),
            `${surfacePath}.description`,
          ),
          regions,
        },
        minimum,
        maximum,
      };
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known presentation response kind");
  }
}

/** Decode the immutable student Question Presentation without changing its issued stage. */
export function decodeIssuedQuestionPresentation(
  value: unknown,
  path = "response",
): QuestionPresentation {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "questionRevision",
    "question_seed",
    "presentationNonce",
    "title",
    "prompt",
    "response",
  ]);
  const nonce = decodeString(field(record, "presentationNonce", path), `${path}.presentationNonce`);
  if (!PRESENTATION_NONCE.test(nonce)) {
    throw new DecodeError(`${path}.presentationNonce`, "32 lowercase hexadecimal characters");
  }
  const presentation = {
    questionRevision: decodeQuestionRevisionReference(
      field(record, "questionRevision", path),
      `${path}.questionRevision`,
      true,
    ),
    question_seed: decodeNonnegativeInteger(
      field(record, "question_seed", path),
      `${path}.question_seed`,
    ),
    presentationNonce: nonce,
    title: decodeQuestionTitle(field(record, "title", path), `${path}.title`),
    prompt: decodeBoundedArray(
      field(record, "prompt", path),
      `${path}.prompt`,
      32,
      (block, blockPath) => decodeQuestionContentBlock(block, blockPath, true),
    ),
    response: issuedQuestionResponseFormat(field(record, "response", path), `${path}.response`),
  } satisfies QuestionPresentation;
  return presentation;
}
