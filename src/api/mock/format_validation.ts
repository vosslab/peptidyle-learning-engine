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
    case "multiBlank": {
      if (response.kind !== "multiBlank") return Promise.resolve(responseKindMismatch());
      const expected = new Set(definition.blanks.map((blank) => blank.id));
      const actual = new Set(response.answers.map((answer) => answer.slot));
      if (
        actual.size !== response.answers.length ||
        actual.size !== expected.size ||
        [...expected].some((slot) => !actual.has(slot))
      ) {
        return Promise.resolve({ violations: [{ kind: "blankSlotsMismatch" }] });
      }
      const tooLong = response.answers.find((answer) => {
        const blank = definition.blanks.find((candidate) => candidate.id === answer.slot);
        return blank !== undefined && [...answer.text].length > blank.maxLength;
      });
      const blank =
        tooLong === undefined
          ? undefined
          : definition.blanks.find((candidate) => candidate.id === tooLong.slot);
      return Promise.resolve({
        violations:
          tooLong === undefined || blank === undefined
            ? []
            : [
                {
                  kind: "textTooLong",
                  maxLength: blank.maxLength,
                  actualLength: [...tooLong.text].length,
                },
              ],
      });
    }
    case "matching": {
      if (response.kind !== "matching") return Promise.resolve(responseKindMismatch());
      const prompts = new Set(definition.prompts.map((prompt) => prompt.id));
      const actualPrompts = new Set(response.matches.map((pair) => pair.prompt));
      if (
        actualPrompts.size !== response.matches.length ||
        actualPrompts.size !== prompts.size ||
        [...prompts].some((prompt) => !actualPrompts.has(prompt))
      ) {
        return Promise.resolve({ violations: [{ kind: "matchingPromptsMismatch" }] });
      }
      const choices = new Set(definition.choices.map((choice) => choice.id));
      const observed = new Set<string>();
      const violations: ResponseFormatViolation[] = [];
      for (const pair of response.matches) {
        if (!choices.has(pair.choice))
          violations.push({ kind: "unknownMatchChoice", choice: pair.choice });
        if (observed.has(pair.choice))
          violations.push({ kind: "duplicateMatchChoice", choice: pair.choice });
        observed.add(pair.choice);
      }
      return Promise.resolve({ violations });
    }
    case "ordering":
      return Promise.resolve(
        response.kind === "ordering"
          ? validateOrdering(definition, response)
          : responseKindMismatch(),
      );
    case "hotspot": {
      if (response.kind !== "hotspot") return Promise.resolve(responseKindMismatch());
      const required =
        definition.selection.kind === "exactlyOne"
          ? 1
          : definition.selection.kind === "exactly"
            ? definition.selection.count
            : undefined;
      if (
        (required !== undefined && response.points.length !== required) ||
        (definition.selection.kind === "atLeastOne" && response.points.length === 0)
      ) {
        return Promise.resolve({
          violations: [
            {
              kind: "selectionCount",
              expected: definition.selection,
              actual: response.points.length,
            },
          ],
        });
      }
      const violations: ResponseFormatViolation[] = [];
      for (const point of response.points) {
        if (point.x < 0 || point.x > 10_000 || point.y < 0 || point.y > 10_000) {
          violations.push({ kind: "hotspotPointOutOfBounds" });
          continue;
        }
        const containing = definition.regions.filter(
          (region) =>
            point.x >= region.x &&
            point.x <= region.x + region.width &&
            point.y >= region.y &&
            point.y <= region.y + region.height,
        );
        if (containing.length !== 1) violations.push({ kind: "hotspotPointOutsideRegion" });
      }
      return Promise.resolve({ violations });
    }
    case "fileUpload":
      if (response.kind !== "fileUpload") {
        return Promise.resolve(responseKindMismatch());
      }
      return Promise.resolve({
        violations: response.objectKey.trim() === "" ? [{ kind: "missingUploadReference" }] : [],
      });
    case "externalTool":
      return Promise.resolve(
        response.kind === "externalTool" ? { violations: [] } : responseKindMismatch(),
      );
  }
}
