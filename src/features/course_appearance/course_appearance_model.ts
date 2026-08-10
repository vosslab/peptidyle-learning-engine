// course_appearance_model.ts - pure instructor draft and validation behavior.

import type { CourseAppearance } from "../../../generated/api/CourseAppearance";
import type { CourseAppearanceUpdate } from "../../../generated/api/CourseAppearanceUpdate";
import type { CourseBannerAlternativeText } from "../../../generated/api/CourseBannerAlternativeText";
import type { CourseBannerCandidateId } from "../../../generated/api/CourseBannerCandidateId";
import type { CourseBannerPresentation } from "../../../generated/api/CourseBannerPresentation";
import type { CourseThemeId } from "../../../generated/api/CourseThemeId";

export const MAX_COURSE_BANNER_BYTES = 2 * 1_024 * 1_024;

const COURSE_BANNER_MEDIA_TYPES = ["image/jpeg", "image/png", "image/webp"] as const;

export interface CourseBannerFileSummary {
  readonly name: string;
  readonly mediaType: string;
  readonly size: number;
}

export type CourseAppearanceBannerDraft =
  | { readonly kind: "none" }
  | {
      readonly kind: "keep";
      readonly presentation: CourseBannerPresentation;
      readonly alternativeText: CourseBannerAlternativeText;
    }
  | {
      readonly kind: "remove";
      readonly presentation: CourseBannerPresentation;
    }
  | {
      readonly kind: "replace";
      readonly file: CourseBannerFileSummary;
      readonly alternativeText: CourseBannerAlternativeText;
    };

export interface CourseAppearanceDraft {
  readonly theme: CourseThemeId;
  readonly banner: CourseAppearanceBannerDraft;
}

export interface CourseAppearanceDraftErrors {
  readonly bannerFile?: string;
  readonly alternativeText?: string;
}

export interface CourseAppearanceDraftValidation {
  readonly valid: boolean;
  readonly errors: CourseAppearanceDraftErrors;
}

function sameAlternativeText(
  left: CourseBannerAlternativeText,
  right: CourseBannerAlternativeText,
): boolean {
  if (left.kind !== right.kind) return false;
  return left.kind === "decorative" || right.kind === "decorative" || left.text === right.text;
}

function initialBannerDraft(appearance: CourseAppearance): CourseAppearanceBannerDraft {
  if (appearance.banner === null) return { kind: "none" };
  return {
    kind: "keep",
    presentation: appearance.banner,
    alternativeText: appearance.banner.alternativeText,
  };
}

/** Starts an editable draft from one exact authorized appearance revision. */
export function initialCourseAppearanceDraft(appearance: CourseAppearance): CourseAppearanceDraft {
  return { theme: appearance.theme, banner: initialBannerDraft(appearance) };
}

export function courseAppearanceDraftWithTheme(
  draft: CourseAppearanceDraft,
  theme: CourseThemeId,
): CourseAppearanceDraft {
  return { ...draft, theme };
}

function editableAlternativeText(
  draft: CourseAppearanceDraft,
  current: CourseAppearance,
): CourseBannerAlternativeText {
  if (draft.banner.kind === "keep" || draft.banner.kind === "replace") {
    return draft.banner.alternativeText;
  }
  return current.banner?.alternativeText ?? { kind: "decorative" };
}

/** Selects a local replacement without uploading or revealing its filename to the server. */
export function courseAppearanceDraftWithReplacement(
  draft: CourseAppearanceDraft,
  current: CourseAppearance,
  file: CourseBannerFileSummary,
): CourseAppearanceDraft {
  return {
    ...draft,
    banner: {
      kind: "replace",
      file,
      alternativeText: editableAlternativeText(draft, current),
    },
  };
}

/** Marks the current banner for removal; saving remains a separate explicit action. */
export function courseAppearanceDraftWithRemoval(
  draft: CourseAppearanceDraft,
  current: CourseAppearance,
): CourseAppearanceDraft {
  if (current.banner === null) return { ...draft, banner: { kind: "none" } };
  return { ...draft, banner: { kind: "remove", presentation: current.banner } };
}

/** Cancels a pending replacement or removal and restores the last server projection. */
export function courseAppearanceDraftWithCurrentBanner(
  draft: CourseAppearanceDraft,
  current: CourseAppearance,
): CourseAppearanceDraft {
  return { ...draft, banner: initialBannerDraft(current) };
}

export function courseAppearanceDraftWithAlternativeText(
  draft: CourseAppearanceDraft,
  alternativeText: CourseBannerAlternativeText,
): CourseAppearanceDraft {
  if (draft.banner.kind === "keep" || draft.banner.kind === "replace") {
    return { ...draft, banner: { ...draft.banner, alternativeText } };
  }
  return draft;
}

function alternativeTextError(alternativeText: CourseBannerAlternativeText): string | undefined {
  if (alternativeText.kind === "decorative") return undefined;
  const characterCount = [...alternativeText.text].length;
  if (characterCount === 0 || characterCount > 160 || alternativeText.text.trim().length === 0) {
    return "Describe the banner in 1 to 160 characters, or mark it decorative.";
  }
  return undefined;
}

/** Validates only helpful client-side constraints; the server remains authoritative. */
export function validateCourseAppearanceDraft(
  draft: CourseAppearanceDraft,
): CourseAppearanceDraftValidation {
  const errors: {
    bannerFile?: string;
    alternativeText?: string;
  } = {};
  if (draft.banner.kind === "replace") {
    const replacement = draft.banner;
    if (replacement.file.size <= 0) {
      errors.bannerFile = "Choose a non-empty JPEG, PNG, or WebP image.";
    } else if (replacement.file.size > MAX_COURSE_BANNER_BYTES) {
      errors.bannerFile = "Choose an image no larger than 2 MiB.";
    } else if (
      !COURSE_BANNER_MEDIA_TYPES.some((mediaType) => mediaType === replacement.file.mediaType)
    ) {
      errors.bannerFile = "Choose a JPEG, PNG, or WebP image.";
    }
    const altError = alternativeTextError(replacement.alternativeText);
    if (altError !== undefined) errors.alternativeText = altError;
  } else if (draft.banner.kind === "keep") {
    const altError = alternativeTextError(draft.banner.alternativeText);
    if (altError !== undefined) errors.alternativeText = altError;
  }
  return { valid: Object.keys(errors).length === 0, errors };
}

function sameCurrentBanner(draft: CourseAppearanceBannerDraft, current: CourseAppearance): boolean {
  if (draft.kind === "none") return current.banner === null;
  if (draft.kind === "remove" || draft.kind === "replace" || current.banner === null) return false;
  return (
    draft.presentation.id === current.banner.id &&
    sameAlternativeText(draft.alternativeText, current.banner.alternativeText)
  );
}

/** Reports whether saving would change the authoritative appearance. */
export function courseAppearanceDraftChanged(
  draft: CourseAppearanceDraft,
  current: CourseAppearance,
): boolean {
  return draft.theme !== current.theme || !sameCurrentBanner(draft.banner, current);
}

/** Builds the exact atomic body after any replacement candidate has uploaded. */
export function courseAppearanceUpdate(
  draft: CourseAppearanceDraft,
  candidate?: CourseBannerCandidateId,
): CourseAppearanceUpdate {
  switch (draft.banner.kind) {
    case "none":
    case "remove":
      return { theme: draft.theme, banner: { kind: "remove" } };
    case "keep":
      return {
        theme: draft.theme,
        banner: { kind: "keep", alternativeText: draft.banner.alternativeText },
      };
    case "replace":
      if (candidate === undefined) {
        throw new Error("A replacement banner must upload before saving appearance");
      }
      return {
        theme: draft.theme,
        banner: {
          kind: "replace",
          candidate,
          alternativeText: draft.banner.alternativeText,
        },
      };
  }
}

export function courseAppearanceBannerWillDisplay(draft: CourseAppearanceDraft): boolean {
  return draft.banner.kind === "keep" || draft.banner.kind === "replace";
}

export function courseAppearanceDraftAlternativeText(
  draft: CourseAppearanceDraft,
): CourseBannerAlternativeText | null {
  return draft.banner.kind === "keep" || draft.banner.kind === "replace"
    ? draft.banner.alternativeText
    : null;
}
