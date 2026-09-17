// Small Sysadmin workspace for the stable global Discipline vocabulary.

import { For, Show, createMemo, createResource, createSignal, type JSX } from "solid-js";

import type { ContentClassificationItem } from "../api/content_classification";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";

const unavailableDisciplines: ReadonlyArray<ContentClassificationItem> = [];

function disciplineFailureCopy(): string {
  return "That Discipline change could not be completed. Check the name and try again.";
}

/** Sysadmin-only creation, rename, retirement, and restoration of stable Disciplines. */
export function ContentDisciplinesPage(): JSX.Element {
  const runtime = useApplicationApi();
  const session = useSessionBootstrap();
  const isSysadmin = createMemo(() => {
    const current = session.state();
    return current.kind === "authenticated" && current.session.account.productRole === "sysadmin";
  });
  const [disciplines, { mutate, refetch }] = createResource<
    ReadonlyArray<ContentClassificationItem>,
    boolean
  >(isSysadmin, async (allowed) =>
    allowed ? runtime.client.listDisciplinesIncludingRetired() : unavailableDisciplines,
  );
  const [newName, setNewName] = createSignal("");
  const [nameByUuid, setNameByUuid] = createSignal<Record<string, string>>({});
  const [busyUuid, setBusyUuid] = createSignal<string | null>(null);
  const [creating, setCreating] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [announcement, setAnnouncement] = createSignal("");

  function displayedName(item: ContentClassificationItem): string {
    return nameByUuid()[item.uuid] ?? item.name;
  }

  function setDisplayedName(uuid: string, name: string): void {
    setNameByUuid((current) => ({ ...current, [uuid]: name }));
  }

  function updateItem(updated: ContentClassificationItem): void {
    mutate((current) =>
      current === undefined
        ? current
        : current.map((item) => (item.uuid === updated.uuid ? updated : item)),
    );
    setNameByUuid((current) => ({ ...current, [updated.uuid]: updated.name }));
  }

  async function create(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (creating()) return;
    const name = newName();
    setCreating(true);
    setError(null);
    try {
      const created = await runtime.client.createDiscipline(name);
      mutate((current) => (current === undefined ? current : [...current, created]));
      setNewName("");
      setAnnouncement(`Discipline ${created.name} created.`);
    } catch {
      setError(disciplineFailureCopy());
    } finally {
      setCreating(false);
    }
  }

  async function rename(item: ContentClassificationItem): Promise<void> {
    if (busyUuid() !== null) return;
    setBusyUuid(item.uuid);
    setError(null);
    try {
      const updated = await runtime.client.renameDiscipline(item.uuid, displayedName(item));
      updateItem(updated);
      setAnnouncement(`Discipline renamed to ${updated.name}.`);
    } catch {
      setError(disciplineFailureCopy());
    } finally {
      setBusyUuid(null);
    }
  }

  async function changeRetirement(item: ContentClassificationItem): Promise<void> {
    if (busyUuid() !== null) return;
    setBusyUuid(item.uuid);
    setError(null);
    try {
      const updated = item.isRetired
        ? await runtime.client.restoreDiscipline(item.uuid)
        : await runtime.client.retireDiscipline(item.uuid);
      updateItem(updated);
      setAnnouncement(
        updated.isRetired
          ? `Discipline ${updated.name} retired.`
          : `Discipline ${updated.name} restored.`,
      );
    } catch {
      setError(disciplineFailureCopy());
    } finally {
      setBusyUuid(null);
    }
  }

  return (
    <section
      class="page"
      data-route-surface="contentDisciplines"
      aria-labelledby="content-disciplines-heading"
    >
      <p class="eyebrow">System administration</p>
      <h1 id="content-disciplines-heading">Disciplines</h1>
      <p class="page-lede">
        Manage the stable shared vocabulary used by Courses and Library Objects. Retired Disciplines
        remain visible for existing content and discovery, but are unavailable for new
        classification choices.
      </p>
      <p class="sr-only" role="status" aria-live="polite">
        {announcement()}
      </p>
      <form
        class="auth-panel"
        aria-busy={creating()}
        novalidate
        onSubmit={(event) => void create(event)}
      >
        <h2>Create Discipline</h2>
        <label for="content-discipline-name">
          Discipline name
          <input
            id="content-discipline-name"
            name="name"
            type="text"
            value={newName()}
            onInput={(event) => setNewName(event.currentTarget.value)}
            maxlength={120}
            required
          />
        </label>
        <button class="primary-action" type="submit" disabled={creating()}>
          {creating() ? "Creating Discipline..." : "Create Discipline"}
        </button>
      </form>
      <Show when={error()}>
        {(message) => (
          <section class="inline-error" role="alert">
            <p>{message()}</p>
          </section>
        )}
      </Show>
      <Show when={disciplines.loading}>
        <p class="loading-state">Loading Disciplines...</p>
      </Show>
      <Show when={disciplines.error !== undefined}>
        <section class="inline-error" role="alert">
          <p>Disciplines could not load. Check your connection and try again.</p>
          <button class="quiet-action" type="button" onClick={() => void refetch()}>
            Retry
          </button>
        </section>
      </Show>
      <Show when={disciplines() !== undefined && disciplines.error === undefined}>
        <section aria-label="Disciplines">
          <For
            each={disciplines()}
            fallback={<p class="empty-state">No Disciplines are available.</p>}
          >
            {(item) => (
              <article class="auth-panel">
                <h2>{item.name}</h2>
                <p>{item.isRetired ? "Retired: unavailable for new choices." : "Active."}</p>
                <label>
                  Discipline name
                  <input
                    type="text"
                    value={displayedName(item)}
                    onInput={(event) => setDisplayedName(item.uuid, event.currentTarget.value)}
                    maxlength={120}
                    disabled={busyUuid() === item.uuid}
                  />
                </label>
                <p>
                  <button
                    class="quiet-action"
                    type="button"
                    disabled={busyUuid() === item.uuid || displayedName(item) === item.name}
                    onClick={() => void rename(item)}
                  >
                    Save name
                  </button>{" "}
                  <button
                    class="quiet-action"
                    type="button"
                    disabled={busyUuid() === item.uuid}
                    onClick={() => void changeRetirement(item)}
                  >
                    {item.isRetired ? "Restore Discipline" : "Retire Discipline"}
                  </button>
                </p>
              </article>
            )}
          </For>
        </section>
      </Show>
    </section>
  );
}
