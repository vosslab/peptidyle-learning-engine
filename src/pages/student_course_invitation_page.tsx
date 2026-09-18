import { A, useParams } from "@solidjs/router";
import { Show, createEffect, createResource, createSignal, onCleanup, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { parseCourseInstanceId } from "../navigation/public_route";

/** Student-owned acceptance of one exact course invitation. */
export function StudentCourseInvitationPage(): JSX.Element {
  const runtime = useApplicationApi();
  const params = useParams();
  function course(): ReturnType<typeof parseCourseInstanceId> {
    return parseCourseInstanceId(params["courseRef"] ?? "");
  }
  const [invitation] = createResource(course, async (reference) => {
    const invitations = await runtime.client.listPendingLiveStudentCourseInvitations();
    return invitations.find((candidate) => candidate.reference === reference) ?? null;
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
    const reference = course();
    if (
      reference === null ||
      busy() ||
      invitation.loading ||
      invitation.error !== undefined ||
      invitation()?.reference !== reference
    )
      return;
    const generation = ++claimGeneration;
    setBusy(true);
    setMessage("");
    setAcceptedCourse(null);
    try {
      const result = await runtime.client.claimLiveCourseInvitation(reference);
      if (generation !== claimGeneration || course() !== reference) return;
      setAcceptedCourse(result.activeStudentMembership ? reference : null);
      setMessage(
        result.activeStudentMembership
          ? "Invitation accepted."
          : "Invitation could not be accepted.",
      );
    } catch {
      if (generation !== claimGeneration || course() !== reference) return;
      setMessage("This invitation could not be accepted.");
    } finally {
      if (generation === claimGeneration) setBusy(false);
    }
  }

  return (
    <section class="page" data-route-surface="studentCourseInvitation">
      <p class="eyebrow">Course invitation</p>
      <Show
        when={acceptedCourse() === course() ? acceptedCourse() : null}
        fallback={
          <>
            <Show when={invitation.loading}>
              <h1>Course invitation</h1>
              <p role="status">Loading course invitation...</p>
            </Show>
            <Show
              when={
                !invitation.loading &&
                (invitation.error !== undefined || invitation()?.reference !== course())
              }
            >
              <h1>Course invitation unavailable</h1>
              <p>This invitation is not available.</p>
              <A class="quiet-link" href="/student/course-invitations">
                Your course invitations
              </A>
            </Show>
            <Show
              when={
                !invitation.loading &&
                invitation.error === undefined &&
                invitation()?.reference === course()
                  ? invitation()
                  : null
              }
            >
              {(context) => (
                <>
                  <h1>{context().longName}</h1>
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
        {(reference) => (
          <>
            <h1>You joined this course</h1>
            <p role="status">Invitation accepted.</p>
            <p>You can now open the course and assigned work.</p>
            <A class="primary-link" href={`/student/courses/${reference()}`}>
              Open course
            </A>
          </>
        )}
      </Show>
    </section>
  );
}
