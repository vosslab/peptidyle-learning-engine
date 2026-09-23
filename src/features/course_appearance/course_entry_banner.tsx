// course_entry_banner.tsx - optional Course entry artwork.

import { Show, type JSX } from "solid-js";

import { useApplicationApi } from "../../api/application_api";
import { useRouteScopeData } from "../../ribbon/route_scope_context";
import { courseBannerImageAlternativeText } from "./course_banner_alternative_text";
import { createCourseBannerUrl } from "./course_banner_delivery";
import { courseRouteView } from "./course_theme_context";

const COURSE_ENTRY_BANNER_STYLES = `
.course-entry-banner-container {
  min-width: 0;
  margin-bottom: var(--ple-space-4, 1rem);
}

.course-entry-banner-frame {
  box-sizing: border-box;
  display: block;
  inline-size: min(100%, 64rem);
  margin-inline: auto;
  overflow: hidden;
  /* HG requires one 5:1 banner, not a page hero. */
  aspect-ratio: 5 / 1;
  border: 1px solid var(--ple-border);
  border-radius: var(--ple-radius-inset, 0.5rem);
}

.course-entry-banner {
  display: block;
  inline-size: 100%;
  block-size: 100%;
  /* The delivery remains complete; the frame never crops authored artwork. */
  object-fit: contain;
  object-position: center;
}
`;

/** Renders the optional entry banner from the authorized Course Route View. */
export function CourseEntryBanner(): JSX.Element {
  const runtime = useApplicationApi();
  const routeData = useRouteScopeData();
  const banner = (): ReturnType<typeof courseRouteView>["appearance"]["banner"] | null => {
    const data = routeData();
    return data === undefined ? null : courseRouteView(data).appearance.banner;
  };
  const deliveryUrl = createCourseBannerUrl(() => banner()?.id ?? null, runtime.client);
  return (
    <Show when={banner() !== null && deliveryUrl() !== undefined}>
      <div class="course-entry-banner-container">
        <style>{COURSE_ENTRY_BANNER_STYLES}</style>
        <div class="course-entry-banner-frame">
          <img
            class="course-entry-banner"
            src={deliveryUrl()}
            alt={courseBannerImageAlternativeText(banner()!.alternativeText)}
            width="1280"
            height="256"
          />
        </div>
      </div>
    </Show>
  );
}
