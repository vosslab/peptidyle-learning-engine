// Student-owned pending course invitation index.

import { A } from "@solidjs/router";
import { createResource, type JSX } from "solid-js";

import type { LiveStudentCourseInvitationSummary } from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import { RecordList, type RecordListState } from "../components/record_list/record_list";
import type { RecordRegion } from "../components/record_list/region_spec";

function invitationRegions(): ReadonlyArray<RecordRegion<LiveStudentCourseInvitationSummary>> {
  return [
    {
      id: "course",
      role: "identity",
      priority: "required",
      width: "minmax(0, 1fr)",
      align: "start",
      content: (invitation): JSX.Element => <h2>{invitation.longName}</h2>,
    },
    {
      id: "details",
      role: "metadata",
      priority: "high",
      width: "minmax(14rem, auto)",
      align: "start",
      content: (invitation): JSX.Element => (
        <>
          <span>Instructor: {invitation.instructorDisplayName}</span>
          <span>
            Term: <time dateTime={invitation.term.startDate}>{invitation.term.startDate}</time>
            {" to "}
            <time dateTime={invitation.term.endDate}>{invitation.term.endDate}</time>
          </span>
        </>
      ),
    },
    {
      id: "action",
      role: "actions",
      priority: "required",
      width: "auto",
      align: "end",
      content: (invitation): JSX.Element => (
        <A class="primary-link" href={`/courses/${invitation.id}/invitation`}>
          Review invitation
        </A>
      ),
    },
  ];
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
        regions={invitationRegions()}
        rows={invitations() ?? []}
        state={invitationListState(invitations.loading, invitations.error !== undefined)}
      />
    </PageFrame>
  );
}
