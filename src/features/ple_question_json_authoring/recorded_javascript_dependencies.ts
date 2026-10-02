// Reviewable inventory of author JavaScript libraries served by PLE.

/** Library names approved for isolated author JavaScript. */
export const RECORDED_EXTERNAL_JAVASCRIPT_DEPENDENCIES = ["rdkit"] as const;

export function isRecordedExternalJavascriptDependency(name: string): boolean {
  return RECORDED_EXTERNAL_JAVASCRIPT_DEPENDENCIES.some((dependency) => dependency === name);
}
