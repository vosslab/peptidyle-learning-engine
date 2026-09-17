// Browser Question ID syntax has one generated Rust-model authority.
import {
  validateCanonicalPublicReference,
  validateCanonicalQuestionIdSyntax,
  type CanonicalPublicReferenceFamily,
} from "../generated/api/QuestionIdSyntaxContract";

export {
  validateCanonicalPublicReference,
  validateCanonicalQuestionIdSyntax,
  type CanonicalPublicReferenceFamily,
} from "../generated/api/QuestionIdSyntaxContract";

/**
 * Accepts one human-entered Question or Pool ID and returns its canonical form.
 *
 * Use only at an explicit paste or text-entry control. Routes, API decoders,
 * storage, and rendered server values must use `validateCanonicalQuestionIdSyntax`
 * instead so they reject alternate spellings rather than silently changing them.
 */
export function normalizeHumanEnteredQuestionId(value: string): string | null {
  const withoutOptionalHyphen =
    value.length === 9 && value.charAt(4) === "-"
      ? value.slice(0, 4) + value.slice(5)
      : value.length === 8
        ? value
        : null;
  if (withoutOptionalHyphen === null || !/^[0-9A-Za-z]+$/u.test(withoutOptionalHyphen)) {
    return null;
  }
  const normalized = withoutOptionalHyphen.toUpperCase().replace(/O/gu, "0").replace(/[IL]/gu, "1");
  return validateCanonicalQuestionIdSyntax(`${normalized.slice(0, 4)}-${normalized.slice(4)}`);
}

/**
 * Accepts one human-entered contiguous public reference and returns its canonical form.
 *
 * This is intentionally limited to text-entry controls. Server output, routes,
 * API decoders, storage, and copied values remain exact and strict.
 */
export function normalizeHumanEnteredPublicReference(
  family: CanonicalPublicReferenceFamily,
  value: string,
): string | null {
  if (!/^[0-9A-Za-z]+$/u.test(value)) return null;
  const normalized = value.toUpperCase().replace(/O/gu, "0").replace(/[IL]/gu, "1");
  return validateCanonicalPublicReference(family, normalized);
}
