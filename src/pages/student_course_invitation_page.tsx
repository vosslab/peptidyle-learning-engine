import { A, useParams } from "@solidjs/router";
import { Show, createEffect, createSignal, onCleanup, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { parseCourseInstanceReference } from "../navigation/public_route";

/** Student-owned acceptance of one exact course invitation. */
export function StudentCourseInvitationPage(): JSX.Element {
  const runtime = useApplicationApi();
  const params = useParams();
  function course(): ReturnType<typeof parseCourseInstanceReference> {
    return parseCourseInstanceReference(params["courseRef"] ?? "");
  }
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [acceptedCourse, setAcceptedCourse] =
    createSignal<ReturnType<typeof parseCourseInstanceReference>>(null);
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
    if (reference === null) return;
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
            <h1>Join this course</h1>
            <p>Accept this invitation to join the course.</p>
            <Show when={message()}>{(value) => <p role="status">{value()}</p>}</Show>
            <button
              class="primary-action"
              type="button"
              disabled={busy() || course() === null}
              onClick={() => void claim()}
            >
              Accept invitation
            </button>
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
