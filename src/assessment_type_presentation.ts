// assessment_type_presentation.ts - fixed learner-visible presentation for Assessment Types.

import { ASSESSMENT_TYPE_VALUES, type AssessmentType } from "../generated/api/AssessmentType";
import type { RibbonGlyphId } from "./ribbon/ribbon_icons";

/** Fixed Human Guidance icon names, each available in the bundled Free Solid sprite. */
export type AssessmentTypeIconId = RibbonGlyphId;

/**
 * The color token is a semantic theme hook. Labels and icons remain the
 * complete Type identifier when color is unavailable.
 */
export interface AssessmentTypePresentation {
  readonly label: string;
  /** Concise Instructor-facing purpose stated by Human Guidance. */
  readonly description: string;
  /** The exact Human Guidance Font Awesome icon name and bundled Ribbon glyph. */
  readonly icon: AssessmentTypeIconId;
  readonly colorToken: `--ple-assessment-type-${string}`;
}

/**
 * The sole browser presentation mapping for the canonical Assessment Type
 * values. `satisfies` makes a new generated Type value a compile-time error.
 */
export const ASSESSMENT_TYPE_PRESENTATIONS = Object.freeze({
  regular_assignment: {
    label: "Regular Assignment",
    description:
      "Practice applying course ideas outside class. Reinforces current learning and can introduce new topics.",
    icon: "pen-to-square",
    colorToken: "--ple-assessment-type-regular-assignment",
  },
  practice_question_assignment: {
    label: "Practice Question Assignment",
    description: "Focused review or study-guide practice using material already covered.",
    icon: "arrows-spin",
    colorToken: "--ple-assessment-type-practice-question-assignment",
  },
  bonus_assignment: {
    label: "Bonus Assignment",
    description: "Optional extra credit.",
    icon: "star",
    colorToken: "--ple-assessment-type-bonus-assignment",
  },
  quiz: {
    label: "Quiz",
    description: "Assesses understanding of recent material.",
    icon: "circle-question",
    colorToken: "--ple-assessment-type-quiz",
  },
  exam: {
    label: "Exam",
    description: "Individual assessment associated with a scheduled exam period.",
    icon: "file-signature",
    colorToken: "--ple-assessment-type-exam",
  },
} as const satisfies Readonly<Record<AssessmentType, AssessmentTypePresentation>>);

export type AssessmentTypeOption = AssessmentTypePresentation & {
  readonly value: AssessmentType;
};

/** Typed select-ready options derived from the generated canonical values. */
export const ASSESSMENT_TYPE_OPTIONS: readonly AssessmentTypeOption[] = Object.freeze(
  ASSESSMENT_TYPE_VALUES.map((value) => ({ value, ...ASSESSMENT_TYPE_PRESENTATIONS[value] })),
);

/** Returns the stable label, icon, and semantic color hook for one Type. */
export function assessmentTypePresentation(type: AssessmentType): AssessmentTypePresentation {
  return ASSESSMENT_TYPE_PRESENTATIONS[type];
}

/** Narrows a browser control value to an emitted canonical Assessment Type. */
export function isAssessmentType(value: string): value is AssessmentType {
  return ASSESSMENT_TYPE_VALUES.some((type) => type === value);
}
