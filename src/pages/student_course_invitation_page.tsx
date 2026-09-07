import { useParams } from "@solidjs/router";
import { Show, createSignal, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { parseCourseInstanceReference } from "../navigation/public_route";

/** Student-owned acceptance of one exact Course Invitation. */
export function StudentCourseInvitationPage(): JSX.Element {
  const runtime = useApplicationApi();
  const params = useParams();
  const course = () => parseCourseInstanceReference(params["courseRef"] ?? "");
  const [busy, setBusy] = createSignal(false);
  const [message, setMessage] = createSignal("");

  async function claim(): Promise<void> {
    const reference = course();
    if (reference === null) return;
    setBusy(true);
    setMessage("");
    try {
      const result = await runtime.client.claimLiveCourseInvitation(reference);
      setMessage(
        result.activeStudentMembership
          ? "Course Invitation accepted. You can now open assigned work."
          : "Course Invitation could not be accepted.",
      );
    } catch {
      setMessage("This Course Invitation could not be accepted.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="page" data-route-surface="studentCourseInvitation">
      <p class="eyebrow">Course Invitation</p>
      <h1>Join this Course Instance</h1>
      <p>Accept your Course Invitation to join this Course Instance as a Student.</p>
      <Show when={message()}>{(value) => <p role="status">{value()}</p>}</Show>
      <button
        class="primary-action"
        type="button"
        disabled={busy() || course() === null}
        onClick={() => void claim()}
      >
        Accept Course Invitation
      </button>
    </section>
  );
}
