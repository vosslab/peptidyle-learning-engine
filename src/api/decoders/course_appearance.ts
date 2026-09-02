// Strict decoders for the course appearance API surface.

import type { CourseAppearanceView } from "../../../generated/api/CourseAppearanceView";
import type { CourseAppearanceUpdate } from "../../../generated/api/CourseAppearanceUpdate";
import type { CourseBannerAlternativeText } from "../../../generated/api/CourseBannerAlternativeText";
import type { CourseBannerUploadReceipt } from "../../../generated/api/CourseBannerUploadReceipt";
import type { CourseBanner } from "../../../generated/api/CourseBanner";
import type { CourseTheme } from "../../../generated/api/CourseTheme";
import {
  DecodeError,
  decodeNullable,
  decodeNonemptyString,
  decodeRecord,
  decodeString,
  decodeStringEnum,
  decodeUuid,
} from "../decoder";
import { field, requireOnlyFields } from "./shared";

const COURSE_THEME_VALUES = [
  "tundra",
  "forest",
  "desert",
  "grass",
  "arctic",
  "ocean",
  "tropical",
  "coral-reef",
  "swamp",
  "underground",
  "salt-marsh",
  "wetland",
  "sea-floor",
  "magma",
  "beach",
] as const satisfies ReadonlyArray<CourseTheme>;

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
  requireOnlyFields(record, path, ["theme", "revision", "banner"]);
  const revision = decodeString(field(record, "revision", path), `${path}.revision`);
  if (!/^[1-9][0-9]*$/u.test(revision) || BigInt(revision) > 9_223_372_036_854_775_807n) {
    throw new DecodeError(`${path}.revision`, "a canonical positive PostgreSQL bigint string");
  }
  return {
    theme: decodeStringEnum(field(record, "theme", path), `${path}.theme`, COURSE_THEME_VALUES),
    revision,
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

/** Strict atomic course-appearance update at the request decoder boundary. */
export function decodeCourseAppearanceUpdate(
  value: unknown,
  path = "request",
): CourseAppearanceUpdate {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["theme", "banner"]);
  const theme = decodeStringEnum(
    field(record, "theme", path),
    `${path}.theme`,
    COURSE_THEME_VALUES,
  );
  const banner = decodeRecord(field(record, "banner", path), `${path}.banner`);
  const kind = decodeStringEnum(field(banner, "kind", `${path}.banner`), `${path}.banner.kind`, [
    "keep",
    "remove",
    "replace",
  ]);
  switch (kind) {
    case "remove":
      requireOnlyFields(banner, `${path}.banner`, ["kind"]);
      return { theme, banner: { kind } };
    case "keep":
      requireOnlyFields(banner, `${path}.banner`, ["kind", "alternativeText"]);
      return {
        theme,
        banner: {
          kind,
          alternativeText: decodeCourseBannerAlternativeText(
            field(banner, "alternativeText", `${path}.banner`),
            `${path}.banner.alternativeText`,
          ),
        },
      };
    case "replace":
      requireOnlyFields(banner, `${path}.banner`, ["kind", "upload", "alternativeText"]);
      return {
        theme,
        banner: {
          kind,
          upload: decodeUuid(field(banner, "upload", `${path}.banner`), `${path}.banner.upload`),
          alternativeText: decodeCourseBannerAlternativeText(
            field(banner, "alternativeText", `${path}.banner`),
            `${path}.banner.alternativeText`,
          ),
        },
      };
  }
}
