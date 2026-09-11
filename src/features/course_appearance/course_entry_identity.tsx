// course_entry_identity.tsx - course title and optional entry-only banner.

import { Show, type JSX } from "solid-js";

import { useApplicationApi } from "../../api/application_api";
import { useRouteScopeData } from "../../ribbon/route_scope_context";
import { courseBannerImageAlternativeText } from "./course_banner_alternative_text";
import { createCourseBannerUrl } from "./course_banner_delivery";
import { courseRouteView } from "./course_theme_context";

const COURSE_ENTRY_IDENTITY_STYLES = `
.course-entry-identity {
  display: grid;
  container-type: inline-size;
  gap: var(--ple-space-2, 0.5rem);
  min-width: 0;
  margin-bottom: var(--ple-space-4, 1rem);
}

.course-entry-identity h1 {
  max-width: 32ch;
}

.course-entry-banner {
  display: block;
  inline-size: min(100%, 75rem);
  block-size: clamp(120px, calc(100cqi / 6), 200px);
  margin-inline: auto;
  aspect-ratio: 6 / 1;
  border: 1px solid var(--ple-border);
  border-radius: var(--ple-radius-inset, 0.5rem);
  object-fit: cover;
}
`;

/** Renders the authorized Course Route View already loaded by the route scope. */
export function CourseEntryIdentity(): JSX.Element {
  const runtime = useApplicationApi();
  const routeData = useRouteScopeData();
  const banner = (): ReturnType<typeof courseRouteView>["appearance"]["banner"] | null => {
    const data = routeData();
    return data === undefined ? null : courseRouteView(data).appearance.banner;
  };
  const deliveryUrl = createCourseBannerUrl(() => banner()?.reference ?? null, runtime.client);
  return (
    <Show when={routeData()} keyed>
      {(data) => {
        const course = courseRouteView(data);
        return (
          <header class="course-entry-identity" data-course-title>
            <style>{COURSE_ENTRY_IDENTITY_STYLES}</style>
            <p class="eyebrow">Course home</p>
            <Show when={banner() !== null && deliveryUrl() !== undefined}>
              <img
                class="course-entry-banner"
                src={deliveryUrl()}
                alt={
                  banner() === null
                    ? ""
                    : courseBannerImageAlternativeText(banner()!.alternativeText)
                }
                width="1200"
                height="200"
              />
            </Show>
            <h1>{course.summary.longName}</h1>
          </header>
        );
      }}
    </Show>
  );
}
