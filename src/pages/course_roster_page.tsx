// course_roster_page.tsx - Course Roster Import and protected roster projection.

import { A, useParams } from "@solidjs/router";
import { For, Show, createEffect, createResource, createSignal, type JSX } from "solid-js";

import "./course_roster_page.css";

import { parseRosterImportRows } from "./roster_import_template";
import { useApplicationApi } from "../api/application_api";
import {
  type CourseInstanceRouteReference,
  parseCourseInstanceReference,
} from "../navigation/public_route";

function stateLabel(state: "invitationPending" | "activeStudent"): string {
  return state === "activeStudent" ? "Active Student" : "Invitation pending";
}

type RosterFeedback = Readonly<{ kind: "success" | "error"; text: string }>;

/** Current Active Instructor surface for an exact Course Instance's roster. */
export function CourseRosterPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  const reference = (): CourseInstanceRouteReference | null =>
    parseCourseInstanceReference(params["courseRef"] ?? "");
  const [roster, { refetch }] = createResource(reference, async (course) => {
    if (course === null) throw new Error("Course Instance reference is invalid");
    return applicationApi.client.getLiveCourseRoster(course);
  });
  const [importText, setImportText] = createSignal("");
  const [message, setMessage] = createSignal<RosterFeedback>();
  const [toolMessage, setToolMessage] = createSignal<RosterFeedback>();
  const [busy, setBusy] = createSignal(false);
  let rosterTools: HTMLDetailsElement | undefined;

  createEffect(() => {
    reference();
    // ASVS 14.3.1/14.3.3: transient FERPA drafts never follow another Course or enter storage.
    setImportText("");
    setMessage(undefined);
    setToolMessage(undefined);
  });

  async function importRoster(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const course = reference();
    if (course === null) return;
    setBusy(true);
    setMessage(undefined);
    setToolMessage(undefined);
    try {
      const entries = parseRosterImportRows(importText());
      await applicationApi.client.importLiveCourseRoster(course, { entries });
      if (reference() !== course) return;
      setImportText("");
      setToolMessage({
        kind: "success",
        text: "Roster import recorded. Course roster names updated; new Students remain pending until they claim their Course Invitation.",
      });
      await refetch();
    } catch (error) {
      if (reference() !== course) return;
      setToolMessage({
        kind: "error",
        text:
          error instanceof Error && error.message.startsWith("Use ")
            ? error.message
            : "The roster import could not be recorded. Check each email, stored roster ID, and Course roster name.",
      });
    } finally {
      setBusy(false);
      if (rosterTools !== undefined) rosterTools.open = true;
    }
  }

  async function revoke(rosterId: string): Promise<void> {
    const course = reference();
    if (course === null) return;
    setBusy(true);
    setMessage(undefined);
    setToolMessage(undefined);
    try {
      await applicationApi.client.revokeLiveCourseRosterEntry(course, rosterId);
      setMessage({
        kind: "success",
        text: "Course access was removed. Protected educational records remain under retention.",
      });
      await refetch();
    } catch {
      setMessage({
        kind: "error",
        text: "This roster entry could not be changed. Reload and try again.",
      });
    } finally {
      setBusy(false);
    }
  }

  async function downloadPendingInvitations(): Promise<void> {
    const course = reference();
    if (course === null) return;
    setBusy(true);
    setMessage(undefined);
    setToolMessage(undefined);
    try {
      const exportBlob = await applicationApi.client.downloadLiveInvitationExport(course);
      const downloadUrl = URL.createObjectURL(exportBlob);
      const link = document.createElement("a");
      link.href = downloadUrl;
      link.download = "ple-invitations.json";
      document.body.append(link);
      link.click();
      link.remove();
      setTimeout(() => URL.revokeObjectURL(downloadUrl), 0);
      setToolMessage({
        kind: "success",
        text: "Pending Course Invitations downloaded for the attended mailer.",
      });
    } catch {
      setToolMessage({
        kind: "error",
        text: "Pending Course Invitations could not be downloaded.",
      });
    } finally {
      setBusy(false);
      if (rosterTools !== undefined) rosterTools.open = true;
    }
  }

  return (
    <section class="page roster-page" data-route-surface="courseRoster">
      <p class="eyebrow">Course Instance roster</p>
      <h1>Students</h1>
      <Show when={message()}>
        {(feedback) => (
          <p
            class={feedback().kind === "error" ? "inline-error" : "roster-feedback-success"}
            role={feedback().kind === "error" ? "alert" : "status"}
          >
            {feedback().text}
          </p>
        )}
      </Show>

      <Show when={roster.loading}>
        <p class="loading-state" role="status">
          Loading course roster...
        </p>
      </Show>
      <Show when={roster.error !== undefined}>
        <section class="route-error" role="alert">
          <h2>Roster unavailable</h2>
          <p>Your current Teaching Team membership cannot load this Course Instance roster.</p>
          <button class="primary-action" type="button" onClick={() => void refetch()}>
            Try again
          </button>
        </section>
      </Show>
      <Show when={!roster.loading && roster.error === undefined && roster()}>
        {(entries) => (
          <section class="roster-section" aria-labelledby="current-roster-heading">
            <h2 id="current-roster-heading">Current roster</h2>
            <Show
              when={entries().length > 0}
              fallback={<p class="empty-state">No roster entries yet.</p>}
            >
              <div class="roster-table-wrap">
                <table class="roster-table">
                  <thead>
                    <tr>
                      <th scope="col">Student</th>
                      <th scope="col">State</th>
                      <th scope="col">Action</th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={entries()}>
                      {(entry) => (
                        <tr>
                          <td>
                            {/* ASVS 1.2.1: Instructor-provided names render only as escaped text. */}
                            <div>{entry.rosterName}</div>
                            <small>{entry.rosterId}</small>
                          </td>
                          <td>{stateLabel(entry.state)}</td>
                          <td>
                            <button
                              class="quiet-action"
                              type="button"
                              disabled={busy()}
                              onClick={() => void revoke(entry.rosterId)}
                            >
                              Remove course access
                            </button>
                          </td>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
              </div>
            </Show>
          </section>
        )}
      </Show>
      <details class="roster-tools" ref={(element) => (rosterTools = element)}>
        <summary>Roster tools</summary>
        <p class="field-help roster-tools-help">
          Import reviewed Student Authentication Email, Course roster ID, and Course roster name
          rows. New entries create pending Course Invitations, not Student Work. To correct a name,
          re-import the same email and stored roster ID with the corrected name. Only the Course
          roster name changes; authentication email and roster identity do not change.
        </p>
        <div class="roster-tools-grid">
          <form class="auth-form" onSubmit={(event) => void importRoster(event)}>
            <h2>Import roster</h2>
            <label for="live-course-roster-import">Email, roster ID, Course roster name</label>
            <textarea
              id="live-course-roster-import"
              rows={4}
              required
              placeholder={
                'synthetic-student@example.edu,synthetic-001,"Example, Synthetic Student"'
              }
              value={importText()}
              onInput={(event) => setImportText(event.currentTarget.value)}
              aria-describedby="live-course-roster-import-help"
            />
            <p id="live-course-roster-import-help" class="field-help">
              One CSV row per line, up to 50 rows; optional header: email,roster_id,roster_name.
              Quote names containing commas and double any quotation marks inside quoted fields.
              Names are Course-local labels, may repeat, and need 1-200 characters without controls.
              Roster IDs are not sign-in credentials. The placeholder is explicitly synthetic.
            </p>
            <button class="primary-action" type="submit" disabled={busy()}>
              Import roster
            </button>
          </form>
          <section aria-labelledby="invitation-export-heading">
            <h2 id="invitation-export-heading">Pending Course Invitations</h2>
            <p class="field-help">
              Download the protected mailer input for pending Course Invitations. This page does not
              send email.
            </p>
            <button
              class="quiet-action"
              type="button"
              disabled={busy()}
              onClick={() => void downloadPendingInvitations()}
            >
              Download pending invitations
            </button>
          </section>
        </div>
        <Show when={toolMessage()}>
          {(feedback) => (
            <p
              class={feedback().kind === "error" ? "inline-error" : "roster-feedback-success"}
              role={feedback().kind === "error" ? "alert" : "status"}
            >
              {feedback().text}
            </p>
          )}
        </Show>
      </details>
      <p>
        <A href="/">Return to Course Instances</A>
      </p>
    </section>
  );
}
