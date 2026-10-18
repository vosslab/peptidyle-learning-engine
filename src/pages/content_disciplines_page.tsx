// Small Sysadmin workspace for the stable global Discipline vocabulary.

import { Show, createMemo, createResource, createSignal, type JSX } from "solid-js";

import type {
  ContentClassificationItem,
  ContentDisciplineRequest,
} from "../api/content_classification";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import { createDisciplineFromRequest } from "../components/discipline_request";
import { PageFrame } from "../components/page_frame/page_frame";
import { UnsavedChangesGuard } from "../components/unsaved_changes_guard";
import type { RecordField } from "../components/data_sheet/data_sheet_fields";
import type { RecordContent } from "../components/data_sheet/record_content";
import { SearchBlock } from "../components/search_block/search_block";
import { SearchControls } from "../components/search_block/search_controls";
import { createLocalCompleteSearch } from "../components/search_block/local_complete_search_state";

const unavailableDisciplines: ReadonlyArray<ContentClassificationItem> = [];
const unavailableRequests: ReadonlyArray<ContentDisciplineRequest> = [];

const requestFields: ReadonlyArray<RecordField<ContentDisciplineRequest, "name", never>> = [
  {
    id: "name",
    label: "Requested Discipline",
    role: "identity",
    value: (item) => item.requestedName,
    sortKey: "name",
    defaultColumn: { priority: 1 },
  },
  {
    id: "requester",
    label: "Requester Account ID",
    value: (item) => item.requestedByAccountId,
    defaultColumn: { priority: 2 },
  },
];

const disciplineFields: ReadonlyArray<RecordField<ContentClassificationItem, "name", "status">> = [
  {
    id: "name",
    label: "Discipline",
    role: "identity",
    value: (item) => item.name,
    sortKey: "name",
    defaultColumn: { priority: 1 },
  },
  {
    id: "status",
    label: "Status",
    value: (item) => (item.isRetired ? "Retired" : "Active"),
    filterKey: "status",
    defaultColumn: { priority: 2 },
  },
];

function disciplineFailureCopy(): string {
  return "That Discipline change could not be completed. Check the name and try again.";
}

/** Sysadmin-only creation, rename, retirement, and restoration of stable Disciplines. */
export function ContentDisciplinesPage(): JSX.Element {
  const runtime = useApplicationApi();
  const session = useSessionBootstrap();
  const isSysadmin = createMemo(() => {
    const current = session.state();
    return current.kind === "authenticated" && current.session.account.userRole === "sysadmin";
  });
  const [disciplines, { mutate, refetch }] = createResource<
    ReadonlyArray<ContentClassificationItem>,
    boolean
  >(isSysadmin, async (allowed) =>
    // ASVS 8.2.1: the Sysadmin route and server own authorization; the client gate avoids eager fetches.
    allowed ? runtime.client.listDisciplinesIncludingRetired() : unavailableDisciplines,
  );
  const [requests, { mutate: mutateRequests, refetch: refetchRequests }] = createResource<
    ReadonlyArray<ContentDisciplineRequest>,
    boolean
  >(isSysadmin, async (allowed) =>
    allowed ? runtime.client.listOpenDisciplineRequests() : unavailableRequests,
  );
  const [newName, setNewName] = createSignal("");
  const [nameByUuid, setNameByUuid] = createSignal<Record<string, string>>({});
  const [busyUuid, setBusyUuid] = createSignal<string | null>(null);
  const [creating, setCreating] = createSignal(false);
  const [pendingNameSave, setPendingNameSave] = createSignal<Promise<boolean>>();
  const [error, setError] = createSignal<string | null>(null);
  const [announcement, setAnnouncement] = createSignal("");

  const requestContent = (item: ContentDisciplineRequest): RecordContent => ({
    title: item.requestedName,
    details: [{ kind: "text", label: "Requester Account ID", value: item.requestedByAccountId }],
    actions: [
      {
        id: "create",
        kind: "command",
        label: "Create this Discipline",
        disabled: busyUuid() !== null || creating(),
        onClick: () => void createFromRequest(item),
      },
      {
        id: "dismiss",
        kind: "command",
        label: "Dismiss request",
        disabled: busyUuid() !== null || creating(),
        onClick: () => void dismissRequest(item),
      },
    ],
  });

  const disciplineContent = (item: ContentClassificationItem): RecordContent => ({
    title: item.name,
    details: [{ kind: "text", label: "Status", value: item.isRetired ? "Retired" : "Active" }],
    actions: [
      {
        id: "save-name",
        kind: "command",
        label: "Save name",
        disabled: creating() || busyUuid() !== null || displayedName(item) === item.name,
        onClick: () => trackNameSave(rename(item)),
      },
      {
        id: "change-retirement",
        kind: "command",
        label: item.isRetired ? "Restore Discipline" : "Retire Discipline",
        disabled: creating() || busyUuid() !== null,
        onClick: () => void changeRetirement(item),
      },
    ],
  });

  const requestSearch = createLocalCompleteSearch(
    {
      rows: requests,
      loading: () => requests.loading,
      error: (): unknown => requests.error,
      refetchSource: refetchRequests,
    },
    {
      fields: requestFields,
      rowId: (item) => item.uuid,
      keyword: (item) => `${item.requestedName} ${item.requestedByAccountId}`,
      content: requestContent,
    },
  );
  const disciplineSearch = createLocalCompleteSearch(
    {
      rows: disciplines,
      loading: () => disciplines.loading,
      error: (): unknown => disciplines.error,
      refetchSource: refetch,
    },
    {
      fields: disciplineFields,
      rowId: (item) => item.uuid,
      keyword: (item) => item.name,
      content: disciplineContent,
    },
  );

  function displayedName(item: ContentClassificationItem): string {
    return nameByUuid()[item.uuid] ?? item.name;
  }

  function setDisplayedName(uuid: string, name: string): void {
    setNameByUuid((current) => ({ ...current, [uuid]: name }));
  }

  const hasUnsavedName = (): boolean =>
    newName().trim().length > 0 ||
    (disciplines() ?? []).some((item) => displayedName(item) !== item.name);

  function trackNameSave(operation: Promise<boolean>): void {
    void setPendingNameSave(operation);
    const clearPending = (): void => {
      if (pendingNameSave() === operation) setPendingNameSave(undefined);
    };
    void operation.then(clearPending, clearPending);
  }

  function updateItem(updated: ContentClassificationItem, savedName = false): void {
    mutate((current) =>
      current === undefined
        ? current
        : current.map((item) => (item.uuid === updated.uuid ? updated : item)),
    );
    if (savedName)
      setNameByUuid((current) => {
        const remaining = { ...current };
        delete remaining[updated.uuid];
        return remaining;
      });
  }

  async function createFromRequest(request: ContentDisciplineRequest): Promise<void> {
    if (busyUuid() !== null || creating()) return;
    setBusyUuid(request.uuid);
    setError(null);
    try {
      const created = await createDisciplineFromRequest(runtime.client, request);
      mutate((current) => (current === undefined ? current : [...current, created]));
      mutateRequests((current) =>
        current === undefined ? current : current.filter((item) => item.uuid !== request.uuid),
      );
      setAnnouncement(`Discipline ${created.name} created.`);
    } catch {
      setError(disciplineFailureCopy());
    } finally {
      setBusyUuid(null);
    }
  }

  async function dismissRequest(request: ContentDisciplineRequest): Promise<void> {
    if (busyUuid() !== null || creating()) return;
    setBusyUuid(request.uuid);
    setError(null);
    try {
      await runtime.client.resolveDisciplineRequest(request.uuid);
      mutateRequests((current) =>
        current === undefined ? current : current.filter((item) => item.uuid !== request.uuid),
      );
      setAnnouncement(`Discipline request ${request.requestedName} dismissed.`);
    } catch {
      setError("That Discipline request could not be dismissed.");
    } finally {
      setBusyUuid(null);
    }
  }

  async function create(event: SubmitEvent): Promise<boolean> {
    event.preventDefault();
    if (creating() || busyUuid() !== null) return false;
    const name = newName();
    setCreating(true);
    setError(null);
    try {
      const created = await runtime.client.createDiscipline(name);
      mutate((current) => (current === undefined ? current : [...current, created]));
      setNewName("");
      setAnnouncement(`Discipline ${created.name} created.`);
      return true;
    } catch {
      setError(disciplineFailureCopy());
      return false;
    } finally {
      setCreating(false);
    }
  }

  async function rename(item: ContentClassificationItem): Promise<boolean> {
    if (busyUuid() !== null || creating()) return false;
    setBusyUuid(item.uuid);
    setError(null);
    try {
      const updated = await runtime.client.renameDiscipline(item.uuid, displayedName(item));
      updateItem(updated, true);
      setAnnouncement(`Discipline renamed to ${updated.name}.`);
      return true;
    } catch {
      setError(disciplineFailureCopy());
      return false;
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
    <PageFrame routeSurface="contentDisciplines">
      <UnsavedChangesGuard
        dirty={hasUnsavedName}
        inFlightSave={pendingNameSave}
        copy={{
          heading: "Discard Discipline name changes?",
          description: "The Discipline names entered here have not been saved.",
        }}
      />
      <p class="sr-only" role="status" aria-live="polite">
        {announcement()}
      </p>
      <form
        class="auth-panel"
        aria-busy={creating()}
        novalidate
        onSubmit={(event) => trackNameSave(create(event))}
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
            disabled={creating()}
          />
        </label>
        <button class="primary-action" type="submit" disabled={creating() || busyUuid() !== null}>
          {creating() ? "Creating Discipline..." : "Create Discipline"}
        </button>
      </form>
      <SearchBlock
        layout="above"
        controls={
          <SearchControls state={requestSearch.state} textLabel="Search Discipline requests" />
        }
        state={requestSearch.state}
        collection={{
          ariaLabel: "Discipline requests",
          emptyState: { title: "No Discipline requests match your search." },
          ...requestSearch.collection,
        }}
      />
      <Show when={error()}>
        {(message) => (
          <section class="inline-error" role="alert">
            <p>{message()}</p>
          </section>
        )}
      </Show>
      <SearchBlock
        layout="above"
        controls={<SearchControls state={disciplineSearch.state} textLabel="Search Disciplines" />}
        state={disciplineSearch.state}
        collection={{
          ariaLabel: "Disciplines",
          emptyState: { title: "No Disciplines match your search." },
          ...disciplineSearch.collection,
          renderBody: (item) => (
            <label>
              Discipline name
              <input
                type="text"
                value={displayedName(item())}
                onInput={(event) => setDisplayedName(item().uuid, event.currentTarget.value)}
                maxlength={120}
                disabled={busyUuid() === item().uuid}
              />
            </label>
          ),
        }}
      />
    </PageFrame>
  );
}
