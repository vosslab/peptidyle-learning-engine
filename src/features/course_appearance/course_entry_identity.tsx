// course_entry_identity.tsx - course title and optional entry-only banner.

import { Show, type JSX } from "solid-js";

import type { CourseBannerAlternativeText } from "../../../generated/api/CourseBannerAlternativeText";
import { useApiRuntime } from "../../api/runtime";
import { createCourseBannerUrl } from "./course_banner_delivery";
import { courseRouteData, useCourseThemeRouteData } from "./course_theme_context";

const COURSE_ENTRY_IDENTITY_STYLES = `
.course-entry-identity {
  display: grid;
  gap: var(--ple-space-2, 0.5rem);
  min-width: 0;
  margin-bottom: var(--ple-space-4, 1rem);
}

.course-entry-identity h1 {
  max-width: 32ch;
}

.course-entry-banner {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 1200 / 328;
  border: 1px solid var(--ple-border);
  border-radius: var(--ple-radius-group, 0.5rem);
  object-fit: cover;
}
`;

function alternativeText(value: CourseBannerAlternativeText): string {
  return value.kind === "informative" ? value.text : "";
}

/** Renders the authorized course projection already loaded by the route scope. */
export function CourseEntryIdentity(): JSX.Element {
  const runtime = useApiRuntime();
  const routeData = useCourseThemeRouteData();
  const banner = routeData === undefined ? null : courseRouteData(routeData).appearance.banner;
  const deliveryUrl = createCourseBannerUrl(() => banner?.id ?? null, runtime.client);
  if (routeData === undefined) return <></>;
  const course = courseRouteData(routeData);
  return (
    <header class="course-entry-identity">
      <style>{COURSE_ENTRY_IDENTITY_STYLES}</style>
      <p class="eyebrow">Course home</p>
      <h1>{course.summary.title}</h1>
      <Show when={banner !== null && deliveryUrl() !== undefined}>
        <img
          class="course-entry-banner"
          src={deliveryUrl()}
          alt={banner === null ? "" : alternativeText(banner.alternativeText)}
          width="1200"
          height="328"
        />
      </Show>
    </header>
  );
}
