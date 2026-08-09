// score_format.ts - consistent, artifact-free score display.

const DISPLAY_DECIMAL_PLACES = 2;

function roundHalfAwayFromZero(value: number, decimalPlaces: number): number {
  if (!Number.isFinite(value)) {
    throw new RangeError("score must be finite");
  }
  const scale = 10 ** decimalPlaces;
  const magnitude = Math.round(Math.abs(value) * scale) / scale;
  const rounded = Math.sign(value) * magnitude;
  return Object.is(rounded, -0) ? 0 : rounded;
}

/** Formats one score value with at most two decimals and no trailing zeroes. */
export function formatScoreValue(value: number): string {
  return roundHalfAwayFromZero(value, DISPLAY_DECIMAL_PLACES).toString();
}

/** Formats an earned/possible point pair without floating-point artifacts. */
export function formatPointScore(earned: number, possible: number): string {
  return `${formatScoreValue(earned)} / ${formatScoreValue(possible)}`;
}

/** Formats a normalized score as a percentage, or a dash when no score exists. */
export function formatPercentScore(score: number | null): string {
  return score === null ? "-" : `${formatScoreValue(score * 100)}%`;
}
