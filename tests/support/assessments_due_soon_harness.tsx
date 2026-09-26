// Mounts the production Instructor Due Soon page with the contract's supplied projection.

import { MemoryRouter, Route, createMemoryHistory, useLocation } from "@solidjs/router";
import type { JSX } from "solid-js";
import { render } from "solid-js/web";

import type { DueSoonAssessments } from "../../src/api/assessment_release";
import { ApplicationApiProvider, type ApplicationApi } from "../../src/api/application_api";
import type { OrdinaryBrowserApiClient } from "../../src/api/client";
import { AssessmentsDueSoonPage } from "../../src/pages/assessments_due_soon_page";
import { RouteScopeProvider } from "../../src/ribbon/route_scope_context";

function HarnessRoot(props: { readonly children?: JSX.Element }): JSX.Element {
  const location = useLocation();
  return (
    <RouteScopeProvider pathname={() => location.pathname}>{props.children}</RouteScopeProvider>
  );
}

/** Mounts the unmodified page with one server-projected Due Soon response. */
export function mountAssessmentsDueSoonHarness(
  target: HTMLElement,
  dueSoon: DueSoonAssessments,
): void {
  const history = createMemoryHistory();
  history.set({ value: "/assessments/due-soon" });
  const applicationApi = {
    client: { listAssessmentsDueSoon: () => Promise.resolve(dueSoon) },
    queries: {
      courseScope: () => Promise.reject(new Error("Course scope is unused by Due Soon.")),
    },
  } as unknown as ApplicationApi<OrdinaryBrowserApiClient>;

  render(
    () => (
      <ApplicationApiProvider applicationApi={applicationApi}>
        <MemoryRouter history={history} root={HarnessRoot}>
          <Route path="/assessments/due-soon" component={AssessmentsDueSoonPage} />
        </MemoryRouter>
      </ApplicationApiProvider>
    ),
    target,
  );
}
