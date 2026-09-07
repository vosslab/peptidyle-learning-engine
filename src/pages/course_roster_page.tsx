// course_roster_page.tsx - M9 Course Roster Import and protected roster projection.

import { A, useParams } from "@solidjs/router";
import { For, Show, createResource, createSignal, type JSX } from "solid-js";

import "./course_roster_page.css";

import type { CourseRosterImportEntry } from "../api/course_roster";
import { useApplicationApi } from "../api/application_api";
import { parseCourseInstanceReference } from "../navigation/public_route";

function parseImportRows(value: string): ReadonlyArray<CourseRosterImportEntry> {
  const lines = value
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
  if (lines.length === 0) throw new Error("Enter at least one email and roster ID row.");
  return lines.map((line) => {
    const fields = line.split(",").map((field) => field.trim());
    const email = fields[0];
    const rosterId = fields[1];
    if (fields.length !== 2 || email === undefined || rosterId === undefined || email === "" || rosterId === "") {
      throw new Error("Use one email,roster_id pair on each row.");
    }
    return { email, rosterId };
  });
}

function stateLabel(state: "invitationPending" | "activeStudent"): string {
  return state === "activeStudent" ? "Active Student" : "Invitation pending";
}

/** Current Active Instructor surface for an exact Course Instance's roster. */
export function CourseRosterPage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const params = useParams();
  const reference = () => parseCourseInstanceReference(params["courseRef"] ?? "");
  const [roster, { refetch }] = createResource(reference, async (course) => {
    if (course === null) throw new Error("Course Instance reference is invalid");
    return applicationApi.client.getLiveCourseRoster(course);
  });
  const [importText, setImportText] = createSignal("");
  const [message, setMessage] = createSignal("");
  const [busy, setBusy] = createSignal(false);

  async function importRoster(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const course = reference();
    if (course === null) return;
    setBusy(true);
    setMessage("");
    try {
      const entries = parseImportRows(importText());
      await applicationApi.client.importLiveCourseRoster(course, { entries });
      setImportText("");
      setMessage("Roster import recorded. Students remain pending until they claim their Course Invitation.");
      await refetch();
    } catch (error) {
      setMessage(
        error instanceof Error && error.message.startsWith("Use ")
          ? error.message
          : "The roster import could not be recorded. Check each email and roster ID.",
      );
    } finally {
      setBusy(false);
    }
  }

  async function revoke(rosterId: string): Promise<void> {
    const course = reference();
    if (course === null) return;
    setBusy(true);
    setMessage("");
    try {
      await applicationApi.client.revokeLiveCourseRosterEntry(course, rosterId);
      setMessage("Course access was removed. Protected educational records remain under retention.");
      await refetch();
    } catch {
      setMessage("This roster entry could not be changed. Reload and try again.");
    } finally {
      setBusy(false);
    }
  }

  return (
    <section class="page roster-page" data-route-surface="courseRoster">
      <p class="eyebrow">Course Instance roster</p>
      <h1>Students</h1>
      <p class="page-lede">
        Import reviewed Student Authentication Email and course roster ID pairs. An import creates
        a pending Course Invitation; it does not create Assignment or Student-work records.
      </p>
      <Show when={message()}>
        {(text) => (
          <p class="inline-error" role="status" aria-live="polite">
            {text()}
          </p>
        )}
      </Show>

      <form class="auth-panel auth-form roster-section" onSubmit={(event) => void importRoster(event)}>
        <h2>Import roster</h2>
        <label for="live-course-roster-import">Email, roster ID</label>
        <textarea
          id="live-course-roster-import"
          rows={6}
          required
          placeholder="student@example.edu,900123456"
          value={importText()}
          onInput={(event) => setImportText(event.currentTarget.value)}
          aria-describedby="live-course-roster-import-help"
        />
        <p id="live-course-roster-import-help" class="field-help">
          One comma-separated pair per line, up to 50 rows. Roster IDs remain course-scoped and
          are not sign-in credentials.
        </p>
        <button class="primary-action" type="submit" disabled={busy()}>
          Import roster
        </button>
      </form>

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
      <Show when={roster()}>
        {(entries) => (
          <section class="roster-section" aria-labelledby="current-roster-heading">
            <h2 id="current-roster-heading">Current roster</h2>
            <Show when={entries().length > 0} fallback={<p class="empty-state">No roster entries yet.</p>}>
              <div class="roster-table-wrap">
                <table class="roster-table">
                  <thead>
                    <tr>
                      <th scope="col">Course roster email</th>
                      <th scope="col">Roster ID</th>
                      <th scope="col">State</th>
                      <th scope="col">Action</th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={entries()}>
                      {(entry) => (
                        <tr>
                          <td>{entry.rosterEmail}</td>
                          <td>
                            <code>{entry.rosterId}</code>
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
      <p>
        <A href="/">Return to Course Instances</A>
      </p>
    </section>
  );
}
