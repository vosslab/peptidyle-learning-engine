// role_home_pages.tsx - explicit Product Role home dashboards and root resolution.

import { A, useNavigate } from "@solidjs/router";
import { createEffect, Show, type JSX } from "solid-js";

import { useSessionBootstrap } from "../auth/session_context";
import { productRoleHomePath } from "../route_contract";
import { CourseListPage } from "./course_list_page";
import { StudentCoursesPage } from "./student_courses_page";
import { useApplicationApi } from "../api/application_api";
import { BlueprintPromotion } from "../features/blueprint_course/blueprint_promotion";

/** Resolves the persistent Peptidyle home link to the signed-in role's dashboard. */
export function RoleHomeResolutionPage(): JSX.Element {
  const session = useSessionBootstrap();
  const navigate = useNavigate();

  createEffect(() => {
    const state = session.state();
    if (state.kind === "authenticated") {
      navigate(productRoleHomePath(state.session.account.productRole), { replace: true });
    }
  });

  return (
    <section class="page" data-route-surface="roleHomeResolution" aria-live="polite">
      <Show
        when={session.state().kind === "loading"}
        fallback={
          <Show
            when={session.state().kind === "error"}
            fallback={
              <>
                <p class="eyebrow">Peptidyle</p>
                <h1>Choose your teaching workspace</h1>
                <p class="page-lede">Sign in to open the dashboard for your Product Role.</p>
                <A class="primary-link" href="/sign-in">
                  Sign in
                </A>
              </>
            }
          >
            <section class="route-error" role="alert">
              <h1>Your workspace could not be opened</h1>
              <p>Check your connection, then try again.</p>
              <button class="primary-action" type="button" onClick={() => void session.retry()}>
                Try again
              </button>
            </section>
          </Show>
        }
      >
        <p class="loading-state">Opening your workspace...</p>
      </Show>
    </section>
  );
}

/** Instructor home keeps Course Instance creation and teaching work together. */
export function InstructorHomePage(): JSX.Element {
  return <CourseListPage />;
}

/** Student home keeps current course selection and released work together. */
export function StudentHomePage(): JSX.Element {
  return <StudentCoursesPage />;
}

/** Sysadmin home presents the backed administration operations. */
export function SysadminHomePage(): JSX.Element {
  const runtime = useApplicationApi();
  return (
    <section class="page" data-route-surface="sysadminHome" aria-labelledby="sysadmin-home-heading">
      <p class="eyebrow">System administration</p>
      <h1 id="sysadmin-home-heading">System administration</h1>
      <p class="page-lede">Open the account or scoped course-support operation you need.</p>
      <nav class="card-grid" aria-label="System administration tools">
        <article class="course-card">
          <h2>Disciplines</h2>
          <p>Manage the stable shared content-classification vocabulary.</p>
          <A class="primary-link" href="/sysadmin/disciplines">
            Open Disciplines
          </A>
        </article>
        <article class="course-card">
          <h2>Library activity</h2>
          <p>Read published Library content and manage its improvement activity.</p>
          <A class="primary-link" href="/library">
            Open Library activity
          </A>
        </article>
        <article class="course-card">
          <h2>Instructor Accounts</h2>
          <p>Create and manage Instructor Account access.</p>
          <A class="primary-link" href="/sysadmin/instructor-accounts">
            Open Instructor Accounts
          </A>
        </article>
        <article class="course-card">
          <h2>Scoped course roster support</h2>
          <p>Open one Instructor-issued support capability.</p>
          <A class="primary-link" href="/sysadmin/support-roster">
            Open scoped roster support
          </A>
        </article>
      </nav>
      <BlueprintPromotion client={runtime.client} />
    </section>
  );
}
