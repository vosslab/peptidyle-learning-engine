// assignment_titles.ts - deterministic public labels for arranged UI-only selection.

const QUESTION_ID = /^[0-9A-HJKMNP-TV-Z]{3}-[0-9A-HJKMNP-TV-Z]{4}$/u;

export function masteryRetryTitle(questionId: string): string {
  if (!QUESTION_ID.test(questionId)) throw new Error("arranged question ID must be canonical");
  return `Peptide mastery retry ${questionId}`;
}

export function examContrastTitle(questionId: string): string {
  if (!QUESTION_ID.test(questionId)) throw new Error("arranged question ID must be canonical");
  return `Peptide exam contrast ${questionId}`;
}
