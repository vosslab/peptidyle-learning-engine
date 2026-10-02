// Shared browser validation for the bounded numeric point-value domain.

import type { AssessmentPointValue } from "../generated/api/AssessmentPointValue";

const POINT_VALUE_PATTERN = /^[0-9]{1,10}(?:\.[0-9]{0,4})?$/u;
const SCALE = 10_000n;
const MAXIMUM_SCALED_VALUE = 1_000_000_000n * SCALE + (SCALE - 1n);

/** ASVS 2.2.1: parses point values within the API's numeric(14,4) range. */
export function assessmentPointValueDraft(value: string): AssessmentPointValue | undefined {
  if (!POINT_VALUE_PATTERN.test(value)) return undefined;
  const [wholeText = "0", fractionText = ""] = value.split(".");
  const whole = BigInt(wholeText);
  const fraction = BigInt(fractionText.padEnd(4, "0") || "0");
  return whole * SCALE + fraction <= MAXIMUM_SCALED_VALUE ? value : undefined;
}
