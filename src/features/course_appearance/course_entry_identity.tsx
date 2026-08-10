// course_entry_identity.tsx - course title and optional entry-only banner.

import { Show, type JSX } from "solid-js";

import type { CourseBannerAlternativeText } from "../../../generated/api/CourseBannerAlternativeText";
import { useApiRuntime } from "../../api/runtime";
import { courseRouteData, useCourseThemeRouteData } from "./course_theme_context";

const COURSE_ENTRY_IDENTITY_STYLES = `
.course-entry-identity {
  display: grid;
  gap: 1rem;
  min-width: 0;
  margin-bottom: 2rem;
}

.course-entry-identity h1 {
  max-width: 24ch;
}

.course-entry-banner {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 1200 / 328;
  border: 1px solid var(--ple-border);
  border-radius: 0.85rem;
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
  if (routeData === undefined) return <></>;
  const course = courseRouteData(routeData);
  return (
    <header class="course-entry-identity">
      <style>{COURSE_ENTRY_IDENTITY_STYLES}</style>
      <p class="eyebrow">Course home</p>
      <h1>{course.summary.title}</h1>
      <Show when={course.appearance.banner} keyed>
        {(banner) => (
          <img
            class="course-entry-banner"
            src={runtime.client.assetUrl(banner.id)}
            alt={alternativeText(banner.alternativeText)}
            width="1200"
            height="328"
          />
        )}
      </Show>
    </header>
  );
}
