// Student-owned pending course invitation index.

import { A } from "@solidjs/router";
import { createResource, type JSX } from "solid-js";

import type { LiveStudentCourseInvitationSummary } from "../api/live_student_course_landing";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import { SearchBlock } from "../features/search/search_block";
import { SearchControls } from "../features/search/search_controls";
import { createLocalCompleteSearch } from "../features/search/local_complete_search_state";
import { byStudentCourseNameThenId, studentInvitationFields } from "./student_collection_fields";
import { type RecordContent } from "../components/record_list/record_list";

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

/** Lists only pending course invitations addressed to the signed-in Student. */
export function StudentCourseInvitationsPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const [invitations, { refetch }] = createResource(async () =>
    byStudentCourseNameThenId(
      await applicationApi.client.listPendingLiveStudentCourseInvitations(),
    ),
  );
  const invitationSearch = createLocalCompleteSearch(
    { resource: invitations, refetchSource: refetch, rows: (value) => value },
    {
      fields: studentInvitationFields,
      rowId: (invitation) => invitation.id,
      keyword: (invitation) =>
        `${invitation.longName} ${invitation.shortName} ${invitation.instructorDisplayName}`,
      content: invitationContent,
    },
  );

  return (
    <PageFrame routeSurface="studentCourseInvitations">
      <A class="quiet-link" href="/student/courses">
        Your courses
      </A>
      <SearchBlock
        layout="above"
        controls={
          <SearchControls state={invitationSearch.state} textLabel="Search course invitations" />
        }
        state={invitationSearch.state}
        collection={{
          ariaLabel: "Course invitations",
          fields: studentInvitationFields,
          query: invitationSearch.query,
          get emptyState() {
            return {
              title:
                invitationSearch.state.query().text || invitationSearch.state.query().filters.size
                  ? "No course invitations match your search."
                  : "You do not have any pending course invitations.",
            };
          },
        }}
      />
    </PageFrame>
  );
}
