import { A, useParams } from "@solidjs/router";
import { Show, createSignal, type JSX } from "solid-js";

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
  const [accepted, setAccepted] = createSignal(false);

  async function claim(): Promise<void> {
    const reference = course();
    if (reference === null) return;
    setBusy(true);
    setMessage("");
    setAccepted(false);
    try {
      const result = await runtime.client.claimLiveCourseInvitation(reference);
      setAccepted(result.activeStudentMembership);
      setMessage(
        result.activeStudentMembership
          ? "Invitation accepted."
          : "Invitation could not be accepted.",
      );
    } catch {
      setMessage("This invitation could not be accepted.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="page" data-route-surface="studentCourseInvitation">
      <p class="eyebrow">Course invitation</p>
      <h1>Join this course</h1>
      <p>Accept this invitation to join the course.</p>
      <Show when={message()}>{(value) => <p role="status">{value()}</p>}</Show>
      <Show when={accepted() ? course() : null}>
        {(reference) => (
          <A class="primary-link" href={`/student/courses/${reference()}`}>
            Open assigned work
          </A>
        )}
      </Show>
      <button
        class="primary-action"
        type="button"
        disabled={busy() || course() === null}
        onClick={() => void claim()}
      >
        Accept invitation
      </button>
    </section>
  );
}
