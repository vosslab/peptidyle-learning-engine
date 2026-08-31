// Strict student-presentation decoding and key-free response-widget projection.

import type { AssetRef } from "../../../generated/api/AssetRef";
import type { ChoiceOption } from "../../../generated/api/ChoiceOption";
import type { HotspotRegion } from "../../../generated/api/HotspotRegion";
import type { PresentationEnvelopeV1 } from "../../../generated/api/PresentationEnvelopeV1";
import type { PresentedBlankV1 } from "../../../generated/api/PresentedBlankV1";
import type { PresentedChoiceV1 } from "../../../generated/api/PresentedChoiceV1";
import type { PresentedHotspotRegionV1 } from "../../../generated/api/PresentedHotspotRegionV1";
import type { QuestionEnvelope } from "../../../generated/api/QuestionEnvelope";
import type { QuestionResponseFormat } from "../../../generated/api/QuestionResponseFormat";
import type { IssuedQuestionResponseFormatV1 } from "../../../generated/api/IssuedQuestionResponseFormatV1";
import type { SelectionCardinality } from "../../../generated/api/SelectionCardinality";
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
  decodeEnvelopeTitle,
  decodeIdentifier,
  decodeQuestionVersionReference,
  decodeSha256,
  field,
  kind,
  requireOnlyFields,
} from "./shared";
import { decodeContentBlock } from "./question_response_format";

const MAX_PRESENTED_ITEMS = 32;
const RENDERED_ITEM_ID = /^[0-9a-f]{4}$/u;
const PRESENTATION_NONCE = /^[0-9a-f]{32}$/u;

function renderedItemId(value: unknown, path: string): string {
  const decoded = decodeString(value, path);
  if (!RENDERED_ITEM_ID.test(decoded)) {
    throw new DecodeError(path, "four lowercase hexadecimal characters");
  }
  return decoded;
}

function presentedChoice(value: unknown, path: string): PresentedChoiceV1 {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["id", "body"]);
  return {
    id: renderedItemId(field(record, "id", path), `${path}.id`),
    body: decodeBoundedArray(field(record, "body", path), `${path}.body`, 32, (block, blockPath) =>
      decodeContentBlock(block, blockPath, true),
    ),
  };
}

function presentedChoices(value: unknown, path: string): PresentedChoiceV1[] {
  const choices = decodeBoundedArray(value, path, MAX_PRESENTED_ITEMS, presentedChoice);
  if (choices.length < 2) throw new DecodeError(path, "at least two presented choices");
  const ids = choices.map((choice) => choice.id);
  if (new Set(ids).size !== ids.length) {
    throw new DecodeError(path, "presented choices with unique IDs");
  }
  return choices;
}

function presentedBlank(value: unknown, path: string): PresentedBlankV1 {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["id", "label", "maxCharacters"]);
  return {
    id: renderedItemId(field(record, "id", path), `${path}.id`),
    label: decodeBoundedArray(
      field(record, "label", path),
      `${path}.label`,
      32,
      (block, blockPath) => decodeContentBlock(block, blockPath, true),
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

function presentedHotspotRegion(value: unknown, path: string): PresentedHotspotRegionV1 {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["label", "x", "y", "width", "height"]);
  const x = normalizedCoordinate(field(record, "x", path), `${path}.x`);
  const y = normalizedCoordinate(field(record, "y", path), `${path}.y`);
  const width = decodePositiveInteger(field(record, "width", path), `${path}.width`);
  const height = decodePositiveInteger(field(record, "height", path), `${path}.height`);
  if (x + width > 10_000 || y + height > 10_000) {
    throw new DecodeError(path, "a rectangle within the normalized 10000 by 10000 surface");
  }
  return {
    label: decodeBoundedArray(
      field(record, "label", path),
      `${path}.label`,
      32,
      (block, blockPath) => decodeContentBlock(block, blockPath, true),
    ),
    x,
    y,
    width,
    height,
  };
}

function assetRef(value: unknown, path: string): AssetRef {
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

function issuedQuestionResponseFormat(value: unknown, path: string): IssuedQuestionResponseFormatV1 {
  const record = decodeRecord(value, path);
  switch (kind(record, path)) {
    case "singleChoice": {
      requireOnlyFields(record, path, ["kind", "choices"]);
      return {
        kind: "singleChoice",
        choices: presentedChoices(field(record, "choices", path), `${path}.choices`),
      };
    }
    case "multipleAnswer": {
      requireOnlyFields(record, path, ["kind", "choices", "minimum", "maximum"]);
      const choices = presentedChoices(field(record, "choices", path), `${path}.choices`);
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
        presentedBlank,
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
      const prompts = presentedChoices(field(record, "prompts", path), `${path}.prompts`);
      const choices = presentedChoices(field(record, "choices", path), `${path}.choices`);
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
        items: presentedChoices(field(record, "items", path), `${path}.items`),
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
          id: renderedItemId(field(surfaceRecord, "id", surfacePath), `${surfacePath}.id`),
          asset: assetRef(field(surfaceRecord, "asset", surfacePath), `${surfacePath}.asset`),
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

function selectionFromBounds(
  minimum: number,
  maximum: number,
  count: number,
  path: string,
): SelectionCardinality {
  if (minimum === 1 && maximum === 1) return { kind: "exactlyOne" };
  if (minimum === 0 && maximum === count) return { kind: "anyNumber" };
  if (minimum === 1 && maximum === count) return { kind: "atLeastOne" };
  if (minimum === maximum) return { kind: "exactly", count: minimum };
  throw new DecodeError(path, "selection bounds supported by the response widget");
}

function choicesForWidget(choices: ReadonlyArray<PresentedChoiceV1>): ChoiceOption[] {
  return choices.map((choice) => ({ id: choice.id, body: choice.body }));
}

function responseForWidget(response: IssuedQuestionResponseFormatV1, path: string): QuestionResponseFormat {
  switch (response.kind) {
    case "singleChoice":
      return {
        kind: "multipleChoice",
        choices: choicesForWidget(response.choices),
        selection: { kind: "exactlyOne" },
      };
    case "multipleAnswer":
      return {
        kind: "multipleChoice",
        choices: choicesForWidget(response.choices),
        selection: selectionFromBounds(
          response.minimum,
          response.maximum,
          response.choices.length,
          path,
        ),
      };
    case "fillIn":
      return { kind: "shortText", matchMode: "exact", maxLength: response.maxCharacters };
    case "multiFillIn":
      return {
        kind: "multiBlank",
        blanks: response.blanks.map((blank) => ({
          id: blank.id,
          label: blank.label,
          matchMode: "exact",
          maxLength: blank.maxCharacters,
        })),
      };
    case "numerical":
      return { kind: "numeric", tolerance: { kind: "exact" }, unit: response.displayedUnit };
    case "matching":
      if (response.reuseChoices) {
        throw new DecodeError(path, "a matching presentation without reusable choices");
      }
      return {
        kind: "matching",
        prompts: choicesForWidget(response.prompts),
        choices: choicesForWidget(response.choices),
      };
    case "ordering":
      return { kind: "ordering", items: choicesForWidget(response.items) };
    case "hotspot": {
      const regions: HotspotRegion[] = response.surface.regions.map((region, index) => ({
        id: `${response.surface.id}-region-${index + 1}`,
        ...region,
      }));
      return {
        kind: "hotspot",
        surface: response.surface.asset,
        description: response.surface.description,
        regions,
        selection: selectionFromBounds(response.minimum, response.maximum, regions.length, path),
      };
    }
  }
}

/** Decode the immutable student presentation and project only its public widget fields. */
export function decodeIssuedPresentationEnvelope(
  value: unknown,
  path = "response",
): QuestionEnvelope {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "questionVersion",
    "seed",
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
    questionVersion: decodeQuestionVersionReference(
      field(record, "questionVersion", path),
      `${path}.questionVersion`,
      true,
    ),
    seed: decodeNonnegativeInteger(field(record, "seed", path), `${path}.seed`),
    presentationNonce: nonce,
    title: decodeEnvelopeTitle(field(record, "title", path), `${path}.title`),
    prompt: decodeBoundedArray(
      field(record, "prompt", path),
      `${path}.prompt`,
      32,
      (block, blockPath) => decodeContentBlock(block, blockPath, true),
    ),
    response: issuedQuestionResponseFormat(field(record, "response", path), `${path}.response`),
  } satisfies PresentationEnvelopeV1;
  return {
    questionVersion: presentation.questionVersion,
    seed: presentation.seed,
    title: presentation.title,
    prompt: presentation.prompt,
    response: responseForWidget(presentation.response, `${path}.response`),
  };
}
