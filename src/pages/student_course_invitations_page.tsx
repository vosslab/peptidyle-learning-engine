// Student-owned pending course invitation index.

import { A } from "@solidjs/router";
import { createResource, For, Show, type JSX } from "solid-js";

import type { LiveStudentCourseInvitationSummary } from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";

function InvitationCard(props: {
  readonly invitation: LiveStudentCourseInvitationSummary;
}): JSX.Element {
  return (
    <article class="course-card">
      <h2>{props.invitation.longName}</h2>
      <A class="primary-link" href={`/courses/${props.invitation.reference}/invitation`}>
        Review invitation
      </A>
    </article>
  );
}

/** Lists only pending course invitations addressed to the signed-in Student. */
export function StudentCourseInvitationsPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const [invitations] = createResource(() =>
    applicationApi.client.listPendingLiveStudentCourseInvitations(),
  );

  return (
    <section class="page" data-route-surface="studentCourseInvitations">
      <p class="eyebrow">Your learning</p>
      <h1>Course invitations</h1>
      <p class="page-lede">Review a pending invitation before joining a course.</p>
      <A class="quiet-link" href="/">
        Your courses
      </A>
      <Show when={invitations.loading}>
        <p class="loading-state">Loading course invitations...</p>
      </Show>
      <Show when={invitations.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Course invitations are unavailable</h2>
          <p>Course invitations are not available right now.</p>
        </section>
      </Show>
      <Show
        when={
          !invitations.loading && invitations.error === undefined && invitations()?.length === 0
        }
      >
        <p class="empty-state">You do not have any pending course invitations.</p>
      </Show>
      <Show when={(invitations()?.length ?? 0) > 0}>
        <div class="card-grid">
          <For each={invitations()}>
            {(invitation) => <InvitationCard invitation={invitation} />}
          </For>
        </div>
      </Show>
    </section>
  );
}
