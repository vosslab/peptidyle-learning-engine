// Browser-safe content and response-definition decoders.

import type { AssetRef } from "../../../generated/api/AssetRef";
import type { ChoiceOption } from "../../../generated/api/ChoiceOption";
import type { ContentBlock } from "../../../generated/api/ContentBlock";
import type { NumericTolerance } from "../../../generated/api/NumericTolerance";
import type { ResponseDefinition } from "../../../generated/api/ResponseDefinition";
import type { SelectionCardinality } from "../../../generated/api/SelectionCardinality";
import {
  DecodeError,
  decodeArray,
  decodeFiniteNumber,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeNullable,
  decodePositiveInteger,
  decodeRecord,
  decodeString,
  decodeStringEnum,
} from "../decoder";
import { decodeIdentifier, decodeSha256, field, kind, requireOnlyFields } from "./shared";

function decodeAssetRef(value: unknown, path: string, strict = false): AssetRef {
  const record = decodeRecord(value, path);
  if (strict) requireOnlyFields(record, path, ["asset", "checksum"]);
  return {
    asset: decodeIdentifier(field(record, "asset", path), `${path}.asset`),
    checksum: decodeSha256(field(record, "checksum", path), `${path}.checksum`),
  } satisfies AssetRef;
}

export function decodeContentBlock(value: unknown, path: string, strict = false): ContentBlock {
  const record = decodeRecord(value, path);
  const block = kind(record, path);
  switch (block) {
    case "text":
      if (strict) requireOnlyFields(record, path, ["kind", "markdown"]);
      return {
        kind: block,
        markdown: decodeString(field(record, "markdown", path), `${path}.markdown`),
      } satisfies ContentBlock;
    case "math":
      if (strict) requireOnlyFields(record, path, ["kind", "latex", "description"]);
      return {
        kind: block,
        latex: decodeString(field(record, "latex", path), `${path}.latex`),
        description: decodeNonemptyString(
          field(record, "description", path),
          `${path}.description`,
        ),
      } satisfies ContentBlock;
    case "image":
      if (strict) requireOnlyFields(record, path, ["kind", "asset", "description"]);
      return {
        kind: block,
        asset: decodeAssetRef(field(record, "asset", path), `${path}.asset`, strict),
        description: decodeNonemptyString(
          field(record, "description", path),
          `${path}.description`,
        ),
      } satisfies ContentBlock;
    case "code":
      if (strict) requireOnlyFields(record, path, ["kind", "language", "source"]);
      return {
        kind: block,
        language: decodeNonemptyString(field(record, "language", path), `${path}.language`),
        source: decodeString(field(record, "source", path), `${path}.source`),
      } satisfies ContentBlock;
    case "table":
      if (strict) requireOnlyFields(record, path, ["kind", "headers", "rows", "description"]);
      return {
        kind: block,
        headers: decodeArray(field(record, "headers", path), `${path}.headers`, decodeString),
        rows: decodeArray(field(record, "rows", path), `${path}.rows`, (row, rowPath) =>
          decodeArray(row, rowPath, decodeString),
        ),
        description: decodeNonemptyString(
          field(record, "description", path),
          `${path}.description`,
        ),
      } satisfies ContentBlock;
    default:
      throw new DecodeError(`${path}.kind`, "a known content-block kind");
  }
}

function decodeChoiceOption(value: unknown, path: string, strict = false): ChoiceOption {
  const record = decodeRecord(value, path);
  if (strict) requireOnlyFields(record, path, ["id", "body"]);
  return {
    id: decodeNonemptyString(field(record, "id", path), `${path}.id`),
    body: decodeArray(field(record, "body", path), `${path}.body`, (block, blockPath) =>
      decodeContentBlock(block, blockPath, strict),
    ),
  } satisfies ChoiceOption;
}

function decodeNormalizedCoordinate(value: unknown, path: string): number {
  const coordinate = decodeNonnegativeInteger(value, path);
  if (coordinate > 10_000) throw new DecodeError(path, "an integer from 0 through 10000");
  return coordinate;
}

function decodeNumericTolerance(value: unknown, path: string, strict = false): NumericTolerance {
  const record = decodeRecord(value, path);
  const tolerance = kind(record, path);
  switch (tolerance) {
    case "exact":
      if (strict) requireOnlyFields(record, path, ["kind"]);
      return { kind: tolerance };
    case "absolute":
      if (strict) requireOnlyFields(record, path, ["kind", "epsilon"]);
      return {
        kind: tolerance,
        epsilon: decodeFiniteNumber(field(record, "epsilon", path), `${path}.epsilon`),
      } satisfies NumericTolerance;
    case "relative":
      if (strict) requireOnlyFields(record, path, ["kind", "fraction"]);
      return {
        kind: tolerance,
        fraction: decodeFiniteNumber(field(record, "fraction", path), `${path}.fraction`),
      } satisfies NumericTolerance;
    case "significantFigures":
      if (strict) requireOnlyFields(record, path, ["kind", "digits"]);
      return {
        kind: tolerance,
        digits: decodePositiveInteger(field(record, "digits", path), `${path}.digits`),
      } satisfies NumericTolerance;
    default:
      throw new DecodeError(`${path}.kind`, "a known numeric tolerance");
  }
}

export function decodeSelectionCardinality(
  value: unknown,
  path: string,
  strict = false,
): SelectionCardinality {
  const record = decodeRecord(value, path);
  const selection = kind(record, path);
  switch (selection) {
    case "exactlyOne":
    case "anyNumber":
    case "atLeastOne":
      if (strict) requireOnlyFields(record, path, ["kind"]);
      return { kind: selection };
    case "exactly":
      if (strict) requireOnlyFields(record, path, ["kind", "count"]);
      return {
        kind: selection,
        count: decodeNonnegativeInteger(field(record, "count", path), `${path}.count`),
      } satisfies SelectionCardinality;
    default:
      throw new DecodeError(`${path}.kind`, "a known selection cardinality");
  }
}

export function decodeResponseDefinition(
  value: unknown,
  path: string,
  strict = false,
): ResponseDefinition {
  const record = decodeRecord(value, path);
  const response = kind(record, path);
  switch (response) {
    case "numeric":
      if (strict) requireOnlyFields(record, path, ["kind", "tolerance", "unit"]);
      return {
        kind: response,
        tolerance: decodeNumericTolerance(
          field(record, "tolerance", path),
          `${path}.tolerance`,
          strict,
        ),
        unit: decodeNullable(field(record, "unit", path), `${path}.unit`, decodeString),
      } satisfies ResponseDefinition;
    case "multipleChoice":
      if (strict) requireOnlyFields(record, path, ["kind", "choices", "selection"]);
      return {
        kind: response,
        choices: decodeArray(
          field(record, "choices", path),
          `${path}.choices`,
          (choice, choicePath) => decodeChoiceOption(choice, choicePath, strict),
        ),
        selection: decodeSelectionCardinality(
          field(record, "selection", path),
          `${path}.selection`,
          strict,
        ),
      } satisfies ResponseDefinition;
    case "shortText":
      if (strict) requireOnlyFields(record, path, ["kind", "matchMode", "maxLength"]);
      return {
        kind: response,
        matchMode: decodeStringEnum(field(record, "matchMode", path), `${path}.matchMode`, [
          "exact",
          "caseInsensitive",
          "normalized",
        ]),
        maxLength: decodeNonnegativeInteger(field(record, "maxLength", path), `${path}.maxLength`),
      } satisfies ResponseDefinition;
    case "multiBlank":
      if (strict) requireOnlyFields(record, path, ["kind", "blanks"]);
      return {
        kind: response,
        blanks: decodeArray(field(record, "blanks", path), `${path}.blanks`, (value, slotPath) => {
          const slot = decodeRecord(value, slotPath);
          if (strict) requireOnlyFields(slot, slotPath, ["id", "label", "matchMode", "maxLength"]);
          return {
            id: decodeNonemptyString(field(slot, "id", slotPath), `${slotPath}.id`),
            label: decodeArray(
              field(slot, "label", slotPath),
              `${slotPath}.label`,
              (block, blockPath) => decodeContentBlock(block, blockPath, strict),
            ),
            matchMode: decodeStringEnum(
              field(slot, "matchMode", slotPath),
              `${slotPath}.matchMode`,
              ["exact", "caseInsensitive", "normalized"],
            ),
            maxLength: decodePositiveInteger(
              field(slot, "maxLength", slotPath),
              `${slotPath}.maxLength`,
            ),
          };
        }),
      } satisfies ResponseDefinition;
    case "matching":
      if (strict) requireOnlyFields(record, path, ["kind", "prompts", "choices"]);
      return {
        kind: response,
        prompts: decodeArray(field(record, "prompts", path), `${path}.prompts`, (item, itemPath) =>
          decodeChoiceOption(item, itemPath, strict),
        ),
        choices: decodeArray(field(record, "choices", path), `${path}.choices`, (item, itemPath) =>
          decodeChoiceOption(item, itemPath, strict),
        ),
      } satisfies ResponseDefinition;
    case "ordering":
      if (strict) requireOnlyFields(record, path, ["kind", "items"]);
      return {
        kind: response,
        items: decodeArray(field(record, "items", path), `${path}.items`, (item, itemPath) =>
          decodeChoiceOption(item, itemPath, strict),
        ),
      } satisfies ResponseDefinition;
    case "hotspot":
      if (strict)
        requireOnlyFields(record, path, ["kind", "surface", "description", "regions", "selection"]);
      return {
        kind: response,
        surface: decodeAssetRef(field(record, "surface", path), `${path}.surface`, strict),
        description: decodeNonemptyString(
          field(record, "description", path),
          `${path}.description`,
        ),
        regions: decodeArray(
          field(record, "regions", path),
          `${path}.regions`,
          (value, regionPath) => {
            const region = decodeRecord(value, regionPath);
            if (strict)
              requireOnlyFields(region, regionPath, ["id", "label", "x", "y", "width", "height"]);
            const x = decodeNormalizedCoordinate(field(region, "x", regionPath), `${regionPath}.x`);
            const y = decodeNormalizedCoordinate(field(region, "y", regionPath), `${regionPath}.y`);
            const width = decodePositiveInteger(
              field(region, "width", regionPath),
              `${regionPath}.width`,
            );
            const height = decodePositiveInteger(
              field(region, "height", regionPath),
              `${regionPath}.height`,
            );
            if (x + width > 10_000 || y + height > 10_000) {
              throw new DecodeError(
                regionPath,
                "a rectangle within the normalized 10000 by 10000 surface",
              );
            }
            return {
              id: decodeNonemptyString(field(region, "id", regionPath), `${regionPath}.id`),
              label: decodeArray(
                field(region, "label", regionPath),
                `${regionPath}.label`,
                (block, blockPath) => decodeContentBlock(block, blockPath, strict),
              ),
              x,
              y,
              width,
              height,
            };
          },
        ),
        selection: decodeSelectionCardinality(
          field(record, "selection", path),
          `${path}.selection`,
          strict,
        ),
      } satisfies ResponseDefinition;
    case "fileUpload":
      if (strict) requireOnlyFields(record, path, ["kind", "maxBytes", "acceptedExtensions"]);
      return {
        kind: response,
        maxBytes: decodePositiveInteger(field(record, "maxBytes", path), `${path}.maxBytes`),
        acceptedExtensions: decodeArray(
          field(record, "acceptedExtensions", path),
          `${path}.acceptedExtensions`,
          decodeNonemptyString,
        ),
      } satisfies ResponseDefinition;
    case "externalTool":
      if (strict) requireOnlyFields(record, path, ["kind"]);
      return { kind: response } satisfies ResponseDefinition;
    default:
      throw new DecodeError(`${path}.kind`, "a known response definition");
  }
}
