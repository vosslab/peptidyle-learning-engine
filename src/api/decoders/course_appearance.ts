// Strict decoders for the course appearance API surface.

import type { CourseAppearanceView } from "../../../generated/api/CourseAppearanceView";
import type { CourseBannerAlternativeText } from "../../../generated/api/CourseBannerAlternativeText";
import type { CourseBannerUploadReceipt } from "../../../generated/api/CourseBannerUploadReceipt";
import type { CourseBanner } from "../../../generated/api/CourseBanner";
import type { CourseBannerUpdate } from "../../../generated/api/CourseBannerUpdate";
import { COURSE_THEME_VALUES } from "../../../generated/api/CourseTheme";
import type { CourseThemeUpdate } from "../../../generated/api/CourseThemeUpdate";
import {
  DecodeError,
  decodeNullable,
  decodeNonemptyString,
  decodeRecord,
  decodeStringEnum,
  decodeUuid,
} from "../decoder";
import { field, requireOnlyFields } from "./shared";

function decodeCourseBannerAlternativeText(
  value: unknown,
  path: string,
): CourseBannerAlternativeText {
  const record = decodeRecord(value, path);
  const kind = decodeStringEnum(field(record, "kind", path), `${path}.kind`, [
    "decorative",
    "informative",
  ]);
  if (kind === "decorative") {
    requireOnlyFields(record, path, ["kind"]);
    return { kind };
  }
  requireOnlyFields(record, path, ["kind", "text"]);
  const text = decodeNonemptyString(field(record, "text", path), `${path}.text`);
  if (text.trim().length === 0 || [...text].length > 160) {
    throw new DecodeError(`${path}.text`, "1 through 160 nonblank characters");
  }
  return { kind, text };
}

function decodeCourseBanner(value: unknown, path: string): CourseBanner {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["reference", "alternativeText"]);
  return {
    reference: decodeUuid(field(record, "reference", path), `${path}.reference`),
    alternativeText: decodeCourseBannerAlternativeText(
      field(record, "alternativeText", path),
      `${path}.alternativeText`,
    ),
  };
}

/** Strict decoder for the browser-safe current Course Appearance View. */
export function decodeCourseAppearanceView(
  value: unknown,
  path = "response",
): CourseAppearanceView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["theme", "banner"]);
  return {
    theme: decodeStringEnum(field(record, "theme", path), `${path}.theme`, COURSE_THEME_VALUES),
    banner: decodeNullable(field(record, "banner", path), `${path}.banner`, decodeCourseBanner),
  };
}

/** Strict receipt for a course-bound, server-normalized temporary banner. */
export function decodeCourseBannerUploadReceipt(
  value: unknown,
  path = "response",
): CourseBannerUploadReceipt {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["upload"]);
  return { upload: decodeUuid(field(record, "upload", path), `${path}.upload`) };
}

/** Strict promotion request; it never carries a Course, Account, or object path. */
export function decodeCourseBannerUpdate(value: unknown, path = "request"): CourseBannerUpdate {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["upload", "alternativeText"]);
  return {
    upload: decodeUuid(field(record, "upload", path), `${path}.upload`),
    alternativeText: decodeCourseBannerAlternativeText(
      field(record, "alternativeText", path),
      `${path}.alternativeText`,
    ),
  };
}

/** Strict independent Course Theme update at the request decoder boundary. */
export function decodeCourseThemeUpdate(value: unknown, path = "request"): CourseThemeUpdate {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["theme"]);
  return {
    theme: decodeStringEnum(field(record, "theme", path), `${path}.theme`, COURSE_THEME_VALUES),
  };
}
