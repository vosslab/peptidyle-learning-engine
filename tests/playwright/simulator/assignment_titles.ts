// assignment_titles.ts - deterministic public labels for arranged UI-only selection.

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/u;

export function masteryRetryTitle(problemId: string): string {
  if (!UUID.test(problemId)) throw new Error("arranged problem ID must be a UUID");
  return `Peptide mastery retry ${problemId}`;
}

export function examContrastTitle(problemId: string): string {
  if (!UUID.test(problemId)) throw new Error("arranged problem ID must be a UUID");
  return `Peptide exam contrast ${problemId}`;
}
