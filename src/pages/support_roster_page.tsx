import { For, Show, createSignal, type JSX } from "solid-js";
import type { CourseRosterEntry } from "../api/course_roster";
import { useApplicationApi } from "../api/application_api";

/** Sysadmin task for one Instructor-issued, exact-capability roster operation. */
export function SupportRosterPage(): JSX.Element {
  const runtime = useApplicationApi();
  const [capabilityId, setCapabilityId] = createSignal("");
  const [entries, setEntries] = createSignal<ReadonlyArray<CourseRosterEntry>>();
  const [error, setError] = createSignal<string>();
  async function open(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    setError(undefined);
    setEntries(undefined);
    try {
      const roster = await runtime.client.readSupportCourseRoster(capabilityId().trim());
      setEntries(roster);
      setCapabilityId("");
    } catch {
      setError(
        "This scoped support operation is unavailable. Confirm the capability has not expired or been revoked.",
      );
    }
  }
  return (
    <section
      class="page"
      data-route-surface="supportRoster"
      aria-labelledby="support-roster-heading"
    >
      <p class="eyebrow">System administration</p>
      <h1 id="support-roster-heading">Scoped course roster support</h1>
      <p class="page-lede">
        Open one Instructor-issued support capability. This operation shows only its registered
        course roster projection.
      </p>
      <form class="auth-panel" onSubmit={(event) => void open(event)}>
        <label for="support-capability-id">
          Support capability ID
          <input
            id="support-capability-id"
            type="password"
            autocomplete="off"
            value={capabilityId()}
            onInput={(event) => setCapabilityId(event.currentTarget.value)}
            required
          />
        </label>
        <button class="primary-action" type="submit">
          Open scoped roster
        </button>
      </form>
      <Show when={error()}>
        {(message) => (
          <p class="inline-error" role="alert">
            {message()}
          </p>
        )}
      </Show>
      <Show when={entries()}>
        {(roster) => (
          <section aria-label="Scoped course roster">
            <h2>Course roster</h2>
            <For each={roster()}>
              {(entry) => (
                <article class="auth-panel">
                  <p>{entry.rosterEmail}</p>
                  <p>Roster ID: {entry.rosterId}</p>
                  <p>
                    State:{" "}
                    {entry.state === "activeStudent" ? "Active Student" : "Invitation pending"}
                  </p>
                </article>
              )}
            </For>
          </section>
        )}
      </Show>
    </section>
  );
}
