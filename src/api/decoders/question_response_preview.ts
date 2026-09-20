// Inspection-only native response shapes. Author IDs and grading fields are rejected.
import type { QuestionResponsePreview } from "../../../generated/api/QuestionResponsePreview";
import type { QuestionPreviewRegion } from "../../../generated/api/QuestionPreviewRegion";
import type { QuestionContentBlock } from "../../../generated/api/QuestionContentBlock";
import {
  DecodeError,
  decodeArray,
  decodeNonnegativeInteger,
  decodeNullable,
  decodeRecord,
  decodeString,
} from "../decoder";
import { field, kind, requireOnlyFields } from "./shared";
import {
  decodeQuestionAssetTuple,
  decodeQuestionContentBlock,
  decodeResponseSelectionRule,
} from "./question_response_format";

function blocks(value: unknown, path: string): QuestionContentBlock[] {
  return decodeArray(value, path, (block, blockPath) =>
    decodeQuestionContentBlock(block, blockPath, true),
  );
}

function region(value: unknown, path: string): QuestionPreviewRegion {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["label", "x", "y", "width", "height"]);
  const decoded = {
    label: blocks(field(record, "label", path), `${path}.label`),
    x: decodeNonnegativeInteger(field(record, "x", path), `${path}.x`),
    y: decodeNonnegativeInteger(field(record, "y", path), `${path}.y`),
    width: decodeNonnegativeInteger(field(record, "width", path), `${path}.width`),
    height: decodeNonnegativeInteger(field(record, "height", path), `${path}.height`),
  };
  if (
    decoded.width === 0 ||
    decoded.height === 0 ||
    decoded.x + decoded.width > 10000 ||
    decoded.y + decoded.height > 10000
  )
    throw new DecodeError(path, "a nonempty region within the normalized image bounds");
  return decoded;
}

export function decodeQuestionResponsePreview(
  value: unknown,
  path: string,
): QuestionResponsePreview {
  const record = decodeRecord(value, path);
  const responseKind = kind(record, path);
  switch (responseKind) {
    case "numeric":
      requireOnlyFields(record, path, ["kind", "unit"]);
      return {
        kind: responseKind,
        unit: decodeNullable(field(record, "unit", path), `${path}.unit`, decodeString),
      };
    case "shortText":
      requireOnlyFields(record, path, ["kind"]);
      return { kind: responseKind };
    case "multipleChoice":
      requireOnlyFields(record, path, ["kind", "choices", "selection"]);
      return {
        kind: responseKind,
        choices: decodeArray(field(record, "choices", path), `${path}.choices`, blocks),
        selection: decodeResponseSelectionRule(
          field(record, "selection", path),
          `${path}.selection`,
          true,
        ),
      };
    case "multiBlank":
      requireOnlyFields(record, path, ["kind", "labels"]);
      return {
        kind: responseKind,
        labels: decodeArray(field(record, "labels", path), `${path}.labels`, blocks),
      };
    case "matching":
      requireOnlyFields(record, path, ["kind", "prompts", "choices"]);
      return {
        kind: responseKind,
        prompts: decodeArray(field(record, "prompts", path), `${path}.prompts`, blocks),
        choices: decodeArray(field(record, "choices", path), `${path}.choices`, blocks),
      };
    case "ordering":
      requireOnlyFields(record, path, ["kind", "items"]);
      return {
        kind: responseKind,
        items: decodeArray(field(record, "items", path), `${path}.items`, blocks),
      };
    case "hotspot":
      requireOnlyFields(record, path, [
        "kind",
        "questionAssetTuple",
        "description",
        "regions",
        "selection",
      ]);
      return {
        kind: responseKind,
        questionAssetTuple: decodeQuestionAssetTuple(
          field(record, "questionAssetTuple", path),
          `${path}.questionAssetTuple`,
          true,
        ),
        description: decodeString(field(record, "description", path), `${path}.description`),
        regions: decodeArray(field(record, "regions", path), `${path}.regions`, region),
        selection: decodeResponseSelectionRule(
          field(record, "selection", path),
          `${path}.selection`,
          true,
        ),
      };
    default:
      throw new DecodeError(`${path}.kind`, "a native Question response preview");
  }
}
