// Browser-side Question ID syntax normalization. The server alone validates
// the HMAC-derived final character before resolving a question.

const QUESTION_ID_ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

export function normalizeQuestionIdSyntax(value: string): string | null {
  const trimmed = value.trim();
  const hyphens = [...trimmed].filter((character) => character === "-").length;
  if (hyphens > 0 && (hyphens !== 1 || trimmed.length !== 8 || trimmed[3] !== "-")) {
    return null;
  }
  const compact = trimmed
    .replace(/-/gu, "")
    .toUpperCase()
    .replace(/O/gu, "0")
    .replace(/[IL]/gu, "1");
  if (
    compact.length !== 7 ||
    [...compact].some((character) => !QUESTION_ID_ALPHABET.includes(character))
  ) {
    return null;
  }
  return `${compact.slice(0, 3)}-${compact.slice(3)}`;
}
