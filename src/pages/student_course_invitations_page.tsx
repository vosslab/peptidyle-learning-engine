// Student-owned pending course invitation index.

import { A } from "@solidjs/router";
import { createResource, For, Show, type JSX } from "solid-js";

import type { LiveStudentCourseInvitationSummary } from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";

function InvitationCard(props: {
  readonly invitation: LiveStudentCourseInvitationSummary;
}): JSX.Element {
  return (
    <article class="course-card">
      <h2>{props.invitation.longName}</h2>
      <p>Instructor: {props.invitation.instructorDisplayName}</p>
      <p>
        Term:{" "}
        <time dateTime={props.invitation.term.startDate}>{props.invitation.term.startDate}</time>
        {" to "}
        <time dateTime={props.invitation.term.endDate}>{props.invitation.term.endDate}</time>
      </p>
      <A class="primary-link" href={`/courses/${props.invitation.id}/invitation`}>
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
    <PageFrame
      routeSurface="studentCourseInvitations"
      eyebrow="Your learning"
      title="Course invitations"
      lede="Review a pending invitation before joining a course."
    >
      <A class="quiet-link" href="/student/courses">
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
    </PageFrame>
  );
}
