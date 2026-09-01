// Browser-owned rendering value for the closed Course Banner accessibility policy.

import type { CourseBannerAlternativeText } from "../../../generated/api/CourseBannerAlternativeText";

/** Supplies the exact image alternative text selected by the closed policy. */
export function courseBannerImageAlternativeText(value: CourseBannerAlternativeText): string {
  return value.kind === "informative" ? value.text : "";
}
