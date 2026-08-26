// curriculum_adoption_model.ts - pure workflow choices for answer-free curriculum adoption.

export type CurriculumAdoptionOperation =
  "blueprint" | "alpha" | "rollover" | "termShift" | "imports";

export type CurriculumAdoptionStage =
  "choose" | "previewing" | "preview" | "applying" | "receipt" | "recovery";

export interface CurriculumAdoptionOperationPresentation {
  readonly label: string;
  readonly description: string;
  readonly requiresSource: boolean;
}

const OPERATION_PRESENTATIONS: Readonly<
  Record<CurriculumAdoptionOperation, CurriculumAdoptionOperationPresentation>
> = {
  blueprint: {
    label: "Add a Blueprint assignment",
    description: "Create one ordinary draft assignment in this live teaching course.",
    requiresSource: true,
  },
  alpha: {
    label: "Create a course from Alpha",
    description: "Create a separate live teaching course from an answer-free Alpha curriculum.",
    requiresSource: true,
  },
  rollover: {
    label: "Rollover this course",
    description: "Create the next teaching course without learner records or issued work.",
    requiresSource: false,
  },
  termShift: {
    label: "Shift this course term",
    description: "Move every unissued assignment schedule together in this live course.",
    requiresSource: false,
  },
  imports: {
    label: "Inspect curriculum imports",
    description: "Review imported assignments, controlled updates, and repair evidence.",
    requiresSource: false,
  },
};

export function curriculumAdoptionOperationPresentation(
  operation: CurriculumAdoptionOperation,
): CurriculumAdoptionOperationPresentation {
  return OPERATION_PRESENTATIONS[operation];
}

export function curriculumAdoptionNextInstruction(
  stage: CurriculumAdoptionStage,
  operation: CurriculumAdoptionOperation,
): string {
  const operationLabel = curriculumAdoptionOperationPresentation(operation).label.toLowerCase();
  switch (stage) {
    case "choose":
      return `Choose the live teaching change, then select its source or target term. ${operationLabel} is selected.`;
    case "previewing":
      return "Preparing the server-owned proposal. Keep this page open while the preview is checked.";
    case "preview":
      return "Review the proposal and its target. Apply only when the destination and schedule are correct.";
    case "applying":
      return "Applying the approved proposal. A retry keeps this operation's idempotency key.";
    case "receipt":
      return "The live change is complete. Open the destination or inspect its import evidence next.";
    case "recovery":
      return "Resolve the named blocker, then regenerate the preview from the preserved choices.";
  }
}

export function curriculumAdoptionOperationNeedsTerm(
  operation: CurriculumAdoptionOperation,
): boolean {
  return operation !== "imports";
}

export function curriculumAdoptionOperationNeedsTitle(
  operation: CurriculumAdoptionOperation,
): boolean {
  return operation === "alpha" || operation === "rollover";
}

/** Preserves a visible selection while changing the proposed teaching operation. */
export function withCurriculumAdoptionOperation<Source>(
  choice: Readonly<{
    readonly operation: CurriculumAdoptionOperation;
    readonly source: Source | undefined;
  }>,
  operation: CurriculumAdoptionOperation,
): { readonly operation: CurriculumAdoptionOperation; readonly source: Source | undefined } {
  return { ...choice, operation };
}

/** Replaces one choice without clearing the other inputs a stale-preview recovery must retain. */
export function withCurriculumAdoptionSource<Source, Choice extends object>(
  choice: Readonly<Choice & { readonly source: Source | undefined }>,
  source: Source,
): Choice & { readonly source: Source } {
  return { ...choice, source };
}
