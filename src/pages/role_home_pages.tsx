// role_home_pages.tsx - explicit Product Role home dashboards and root resolution.

import { A, useNavigate } from "@solidjs/router";
import { createEffect, Show, type JSX } from "solid-js";

import { useSessionBootstrap } from "../auth/session_context";
import { productRoleHomePath } from "../route_contract";
import { CourseListPage } from "./course_list_page";
import { StudentAllCourseworkPage } from "./student_course_landing_page";
import { useApplicationApi } from "../api/application_api";
import { BlueprintPromotion } from "../features/blueprint_course/blueprint_promotion";
import { PageFrame } from "../components/page_frame";

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
    <PageFrame
      routeSurface="roleHomeResolution"
      title={
        session.state().kind === "error"
          ? "Your workspace could not be opened"
          : "Choose your teaching workspace"
      }
      eyebrow="Peptidyle"
      lede={
        session.state().kind === "error"
          ? undefined
          : "Sign in to open the dashboard for your Product Role."
      }
    >
      <Show
        when={session.state().kind === "loading"}
        fallback={
          <Show
            when={session.state().kind === "error"}
            fallback={
              <>
                <A class="primary-link" href="/sign-in">
                  Sign in
                </A>
              </>
            }
          >
            <section class="route-error" role="alert">
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
    </PageFrame>
  );
}

/** Instructor home keeps Course Instance creation and teaching work together. */
export function InstructorHomePage(): JSX.Element {
  return <CourseListPage />;
}

/** Opens the Student's Coursework across all enrolled Courses. */
export function StudentHomePage(): JSX.Element {
  return <StudentAllCourseworkPage />;
}

/** Sysadmin home presents the backed administration operations. */
export function SysadminHomePage(): JSX.Element {
  const runtime = useApplicationApi();
  return (
    <PageFrame
      routeSurface="sysadminHome"
      headingId="sysadmin-home-heading"
      eyebrow="System administration"
      title="System administration"
      lede="Open the account or scoped course-support operation you need."
    >
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
    </PageFrame>
  );
}
