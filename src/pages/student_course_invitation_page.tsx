import { A, useParams } from "@solidjs/router";
import { Show, createEffect, createResource, createSignal, onCleanup, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import { parseCourseInstanceId } from "../navigation/public_route";

/** Student-owned acceptance of one exact course invitation. */
export function StudentCourseInvitationPage(): JSX.Element {
  const runtime = useApplicationApi();
  const params = useParams();
  function course(): ReturnType<typeof parseCourseInstanceId> {
    return parseCourseInstanceId(params["courseInstanceId"] ?? "");
  }
  const [invitation] = createResource(course, async (courseInstanceId) => {
    const invitations = await runtime.client.listPendingLiveStudentCourseInvitations();
    return invitations.find((candidate) => candidate.id === courseInstanceId) ?? null;
  });
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [acceptedCourse, setAcceptedCourse] =
    createSignal<ReturnType<typeof parseCourseInstanceId>>(null);
  let claimGeneration = 0;

  createEffect(() => {
    course();
    claimGeneration += 1;
    setBusy(false);
    setMessage("");
    setAcceptedCourse(null);
    onCleanup(() => {
      claimGeneration += 1;
    });
  });

  async function claim(): Promise<void> {
    const courseInstanceId = course();
    if (
      courseInstanceId === null ||
      busy() ||
      invitation.loading ||
      invitation.error !== undefined ||
      invitation()?.id !== courseInstanceId
    )
      return;
    const generation = ++claimGeneration;
    setBusy(true);
    setMessage("");
    setAcceptedCourse(null);
    try {
      const result = await runtime.client.claimLiveCourseInvitation(courseInstanceId);
      if (generation !== claimGeneration || course() !== courseInstanceId) return;
      setAcceptedCourse(result.activeStudentMembership ? courseInstanceId : null);
      setMessage(
        result.activeStudentMembership
          ? "Invitation accepted."
          : "Invitation could not be accepted.",
      );
    } catch {
      if (generation !== claimGeneration || course() !== courseInstanceId) return;
      setMessage("This invitation could not be accepted.");
    } finally {
      if (generation === claimGeneration) setBusy(false);
    }
  }

  return (
    <PageFrame
      routeSurface="studentCourseInvitation"
      eyebrow="Course invitation"
      title={acceptedCourse() === course() ? "You joined this course" : "Course invitation"}
    >
      <Show
        when={acceptedCourse() === course() ? acceptedCourse() : null}
        fallback={
          <>
            <Show when={invitation.loading}>
              <p role="status">Loading course invitation...</p>
            </Show>
            <Show
              when={
                !invitation.loading &&
                (invitation.error !== undefined || invitation()?.id !== course())
              }
            >
              <h2>Course invitation unavailable</h2>
              <p>This invitation is not available.</p>
              <A class="quiet-link" href="/student/course-invitations">
                Your course invitations
              </A>
            </Show>
            <Show
              when={
                !invitation.loading &&
                invitation.error === undefined &&
                invitation()?.id === course()
                  ? invitation()
                  : null
              }
            >
              {(context) => (
                <>
                  <h2>{context().longName}</h2>
                  <p>Instructor: {context().instructorDisplayName}</p>
                  <p>
                    Term:{" "}
                    <time dateTime={context().term.startDate}>{context().term.startDate}</time>
                    {" to "}
                    <time dateTime={context().term.endDate}>{context().term.endDate}</time>
                  </p>
                  <p>Accept this invitation to join the course.</p>
                  <Show when={message()}>{(value) => <p role="status">{value()}</p>}</Show>
                  <button
                    class="primary-action"
                    type="button"
                    disabled={busy()}
                    onClick={() => void claim()}
                  >
                    Accept invitation
                  </button>
                </>
              )}
            </Show>
          </>
        }
      >
        {(courseInstanceId) => (
          <>
            <p role="status">Invitation accepted.</p>
            <p>You can now open the course and assigned work.</p>
            <A class="primary-link" href={`/student/courses/${courseInstanceId()}`}>
              Open course
            </A>
          </>
        )}
      </Show>
    </PageFrame>
  );
}
