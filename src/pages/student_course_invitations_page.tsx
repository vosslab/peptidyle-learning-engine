// Student-owned pending course invitation index.

import { A } from "@solidjs/router";
import { createResource, type JSX } from "solid-js";

import type { LiveStudentCourseInvitationSummary } from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import {
  RecordList,
  type RecordContent,
  type RecordListState,
} from "../components/record_list/record_list";

function invitationContent(invitation: LiveStudentCourseInvitationSummary): RecordContent {
  return {
    title: invitation.longName,
    details: [
      { kind: "text", label: "Instructor", value: invitation.instructorDisplayName },
      {
        kind: "time",
        label: "Term starts",
        value: invitation.term.startDate,
        dateTime: invitation.term.startDate,
      },
      {
        kind: "time",
        label: "Term ends",
        value: invitation.term.endDate,
        dateTime: invitation.term.endDate,
      },
    ],
    actions: [
      {
        id: "review-invitation",
        kind: "link",
        label: "Review invitation",
        href: `/courses/${invitation.id}/invitation`,
        primary: true,
      },
    ],
  };
}

function invitationListState(loading: boolean, unavailable: boolean): RecordListState {
  if (loading) return { kind: "loading", label: "Loading course invitations..." };
  if (unavailable) {
    return {
      kind: "error",
      title: "Course invitations are unavailable",
      message: "Course invitations are not available right now.",
    };
  }
  return { kind: "ready" };
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
      <RecordList
        ariaLabel="Course invitations"
        emptyState={{ title: "You do not have any pending course invitations." }}
        recordId={(invitation) => invitation.id}
        content={invitationContent}
        rows={invitations() ?? []}
        state={invitationListState(invitations.loading, invitations.error !== undefined)}
      />
    </PageFrame>
  );
}
