// format_validation.ts - browser mock of the server format-validation endpoint.

import type { ResponseDefinition } from "../../../generated/api/ResponseDefinition";
import type { StudentResponse } from "../../../generated/api/StudentResponse";
import type { ResponseFormatReport, ResponseFormatViolation } from "../../wasm/index";

function responseKindMismatch(): ResponseFormatReport {
  return { violations: [{ kind: "responseKindMismatch" }] };
}

function selectionCountMatches(
  definition: Extract<ResponseDefinition, { kind: "multipleChoice" }>,
  actual: number,
): boolean {
  switch (definition.selection.kind) {
    case "exactlyOne":
      return actual === 1;
    case "exactly":
      return actual === definition.selection.count;
    case "anyNumber":
      return true;
    case "atLeastOne":
      return actual >= 1;
  }
}

function validateMultipleChoice(
  definition: Extract<ResponseDefinition, { kind: "multipleChoice" }>,
  response: Extract<StudentResponse, { kind: "multipleChoice" }>,
): ResponseFormatReport {
  const violations: ResponseFormatViolation[] = [];
  if (!selectionCountMatches(definition, response.selected.length)) {
    violations.push({
      kind: "selectionCount",
      expected: definition.selection,
      actual: response.selected.length,
    });
  }

  const available = new Set(definition.choices.map((choice) => choice.id));
  const observed = new Set<string>();
  for (const choice of response.selected) {
    if (observed.has(choice)) {
      violations.push({ kind: "duplicateChoice", choice });
    }
    observed.add(choice);
    if (!available.has(choice)) {
      violations.push({ kind: "unknownChoice", choice });
    }
  }
  return { violations };
}

function validateOrdering(
  definition: Extract<ResponseDefinition, { kind: "ordering" }>,
  response: Extract<StudentResponse, { kind: "ordering" }>,
): ResponseFormatReport {
  const expected = new Set(definition.items.map((item) => item.id));
  const actual = new Set(response.order);
  const exactPermutation =
    expected.size === definition.items.length &&
    actual.size === response.order.length &&
    response.order.length === definition.items.length &&
    [...expected].every((item) => actual.has(item));
  return exactPermutation
    ? { violations: [] }
    : { violations: [{ kind: "orderingItemsMismatch" }] };
}

/**
 * Key-free validation used only when the reference UI is running against mocks.
 * It mirrors `crates/domain/src/validation.rs`; it never judges correctness.
 */
export function validateResponseFormatInMock(
  definition: ResponseDefinition,
  response: StudentResponse,
): Promise<ResponseFormatReport> {
  switch (definition.kind) {
    case "numeric":
      if (response.kind !== "numeric") {
        return Promise.resolve(responseKindMismatch());
      }
      return Promise.resolve({
        violations: Number.isFinite(response.value) ? [] : [{ kind: "numericNotFinite" }],
      });
    case "multipleChoice":
      return Promise.resolve(
        response.kind === "multipleChoice"
          ? validateMultipleChoice(definition, response)
          : responseKindMismatch(),
      );
    case "shortText":
      if (response.kind !== "shortText") {
        return Promise.resolve(responseKindMismatch());
      }
      return Promise.resolve({
        violations:
          [...response.text].length > definition.maxLength
            ? [
                {
                  kind: "textTooLong",
                  maxLength: definition.maxLength,
                  actualLength: [...response.text].length,
                },
              ]
            : [],
      });
    case "ordering":
      return Promise.resolve(
        response.kind === "ordering"
          ? validateOrdering(definition, response)
          : responseKindMismatch(),
      );
    case "fileUpload":
      if (response.kind !== "fileUpload") {
        return Promise.resolve(responseKindMismatch());
      }
      return Promise.resolve({
        violations: response.objectKey.trim() === "" ? [{ kind: "missingUploadReference" }] : [],
      });
  }
}
