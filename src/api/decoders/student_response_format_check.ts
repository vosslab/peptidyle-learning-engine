// Strict browser decoder for the answer-free Student Response Format Check contract.

import type { ResponseItemReference } from "../../../generated/api/ResponseItemReference";
import type { ResponseSelectionRule } from "../../../generated/api/ResponseSelectionRule";
import {
  DecodeError,
  decodeArray,
  decodeNonemptyString,
  decodeNonnegativeInteger,
  decodeRecord,
} from "../decoder";
import { decodeResponseSelectionRule } from "./question_response_format";
import { field, kind, requireOnlyFields } from "./shared";

export type StudentResponseFormatIssue =
  | { readonly kind: "responseKindMismatch" }
  | { readonly kind: "numericNotFinite" }
  | {
      readonly kind: "selectionCount";
      readonly expected: ResponseSelectionRule;
      readonly actual: number;
    }
  | { readonly kind: "duplicateChoice"; readonly choice: ResponseItemReference }
  | { readonly kind: "unknownChoice"; readonly choice: ResponseItemReference }
  | {
      readonly kind: "textTooLong";
      readonly maxLength: number;
      readonly actualLength: number;
    }
  | { readonly kind: "blankSlotsMismatch" }
  | { readonly kind: "matchingPromptsMismatch" }
  | { readonly kind: "duplicateMatchChoice"; readonly choice: ResponseItemReference }
  | { readonly kind: "unknownMatchChoice"; readonly choice: ResponseItemReference }
  | { readonly kind: "orderingItemsMismatch" }
  | { readonly kind: "duplicateHotspotRegion"; readonly region: ResponseItemReference }
  | { readonly kind: "unknownHotspotRegion"; readonly region: ResponseItemReference };

export interface StudentResponseFormatCheck {
  readonly issues: ReadonlyArray<StudentResponseFormatIssue>;
}

/**
 * ASVS 1.5.2 and 2.2.1: allow-list every untrusted response shape before use.
 */
export function decodeStudentResponseFormatIssue(
  value: unknown,
  path: string,
): StudentResponseFormatIssue {
  const record = decodeRecord(value, path);
  const issue = kind(record, path);
  switch (issue) {
    case "responseKindMismatch":
    case "numericNotFinite":
    case "blankSlotsMismatch":
    case "matchingPromptsMismatch":
    case "orderingItemsMismatch":
      requireOnlyFields(record, path, ["kind"]);
      return { kind: issue };
    case "selectionCount": {
      requireOnlyFields(record, path, ["kind", "expected", "actual"]);
      const decoded = {
        kind: issue,
        expected: decodeResponseSelectionRule(
          field(record, "expected", path),
          `${path}.expected`,
          true,
        ),
        actual: decodeNonnegativeInteger(field(record, "actual", path), `${path}.actual`),
      } satisfies StudentResponseFormatIssue;
      return decoded;
    }
    case "duplicateChoice":
    case "unknownChoice":
    case "duplicateMatchChoice":
    case "unknownMatchChoice": {
      requireOnlyFields(record, path, ["kind", "choice"]);
      const decoded = {
        kind: issue,
        choice: decodeNonemptyString(field(record, "choice", path), `${path}.choice`),
      } satisfies StudentResponseFormatIssue;
      return decoded;
    }
    case "textTooLong": {
      requireOnlyFields(record, path, ["kind", "maxLength", "actualLength"]);
      const decoded = {
        kind: issue,
        maxLength: decodeNonnegativeInteger(field(record, "maxLength", path), `${path}.maxLength`),
        actualLength: decodeNonnegativeInteger(
          field(record, "actualLength", path),
          `${path}.actualLength`,
        ),
      } satisfies StudentResponseFormatIssue;
      return decoded;
    }
    case "duplicateHotspotRegion":
    case "unknownHotspotRegion": {
      requireOnlyFields(record, path, ["kind", "region"]);
      const decoded = {
        kind: issue,
        region: decodeNonemptyString(field(record, "region", path), `${path}.region`),
      } satisfies StudentResponseFormatIssue;
      return decoded;
    }
    default:
      throw new DecodeError(`${path}.kind`, "a known Student Response Format Issue kind");
  }
}

export function decodeStudentResponseFormatCheck(
  value: unknown,
  path = "response",
): StudentResponseFormatCheck {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["issues"]);
  const decoded = {
    issues: decodeArray(
      field(record, "issues", path),
      `${path}.issues`,
      decodeStudentResponseFormatIssue,
    ),
  } satisfies StudentResponseFormatCheck;
  return decoded;
}
