// flat_numeric_model.ts - private authoring validation for numeric responses.

import type {
  FlatQuestionNumericResponse,
  FlatQuestionNumericTolerance,
} from "./flat_question_source";

export type FlatNumericValidation = {
  readonly answer: number | null;
  readonly valid: boolean;
  readonly issues: Readonly<Record<string, string>>;
};

const NUMERIC_LITERAL = /^[+-]?(?:(?:\d+(?:\.\d*)?)|(?:\.\d+))(?:[eE][+-]?\d+)?$/u;

/** Parses only complete decimal/scientific literals; the editor retains the original literal separately. */
export function parseNumericLiteral(literal: string): number | null {
  const trimmed = literal.trim();
  if (!NUMERIC_LITERAL.test(trimmed)) return null;
  const value = Number(trimmed);
  return Number.isFinite(value) ? value : null;
}

export function numericToleranceField(
  tolerance: FlatQuestionNumericTolerance,
): "epsilon" | "fraction" | "digits" | null {
  if (tolerance.kind === "absolute") return "epsilon";
  if (tolerance.kind === "relative") return "fraction";
  if (tolerance.kind === "significantFigures") return "digits";
  return null;
}

export function validateNumericResponse(
  answerLiteral: string,
  tolerance: FlatQuestionNumericTolerance,
  unit: string | null,
): FlatNumericValidation {
  const issues: Record<string, string> = {};
  const answer = parseNumericLiteral(answerLiteral);
  if (answer === null) issues.answer = "Enter a finite numeric value, such as 6.02e23.";
  if (
    tolerance.kind === "absolute" &&
    (!Number.isFinite(tolerance.epsilon) || tolerance.epsilon < 0)
  ) {
    issues.epsilon = "Absolute tolerance must be a finite value of zero or greater.";
  }
  if (
    tolerance.kind === "relative" &&
    (!Number.isFinite(tolerance.fraction) || tolerance.fraction < 0)
  ) {
    issues.fraction = "Relative tolerance must be a finite fraction of zero or greater.";
  }
  if (
    tolerance.kind === "significantFigures" &&
    (!Number.isSafeInteger(tolerance.digits) || tolerance.digits < 1 || tolerance.digits > 255)
  ) {
    issues.digits = "Significant figures must be a whole number from 1 through 255.";
  }
  if (unit !== null && unit.trim() === "")
    issues.unit = "Leave the unit blank or enter a short unit label.";
  return { answer, valid: Object.keys(issues).length === 0, issues };
}

export function setNumericToleranceKind(
  kind: FlatQuestionNumericTolerance["kind"],
): FlatQuestionNumericTolerance {
  switch (kind) {
    case "exact":
      return { kind };
    case "absolute":
      return { kind, epsilon: 0 };
    case "relative":
      return { kind, fraction: 0 };
    case "significantFigures":
      return { kind, digits: 1 };
  }
}

export function setNumericToleranceValue(
  tolerance: FlatQuestionNumericTolerance,
  value: number,
): FlatQuestionNumericTolerance {
  if (tolerance.kind === "absolute") return { ...tolerance, epsilon: value };
  if (tolerance.kind === "relative") return { ...tolerance, fraction: value };
  if (tolerance.kind === "significantFigures") return { ...tolerance, digits: value };
  return tolerance;
}

export function numericResponseFromAuthoring(
  response: FlatQuestionNumericResponse,
  answerLiteral: string,
  tolerance: FlatQuestionNumericTolerance,
  unit: string | null,
): FlatQuestionNumericResponse | null {
  const validation = validateNumericResponse(answerLiteral, tolerance, unit);
  return validation.answer === null || !validation.valid
    ? null
    : { ...response, answer: validation.answer, tolerance, unit };
}
