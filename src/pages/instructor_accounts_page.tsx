// Live Sysadmin Instructor Account workflow; it never renders Authentication Email.

import { Show, createEffect, createSignal, type JSX } from "solid-js";

import type {
  InstructorAccountBrowse,
  InstructorAccountList,
  InstructorAccountState,
  InstructorAccountSummary,
} from "../api/instructor_account";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import { PageFrame } from "../components/page_frame/page_frame";
import { UnsavedChangesGuard } from "../components/unsaved_changes_guard";
import type { RecordContent } from "../components/data_sheet/record_content";
import type {
  RecordField,
  DataSheetQuery,
  RecordSort,
} from "../components/data_sheet/data_sheet_fields";
import { SearchBlock } from "../components/search_block/search_block";
import { SearchControls } from "../components/search_block/search_controls";
import { createSearchState } from "../components/search_block/search_state";
import { createDisplayDateTimeFormatter } from "../format_datetime";
import { PROVIDED_AVATAR_CATALOG } from "../features/profile_avatar/avatar_catalog_generated";
import { formatSignInLabel } from "./instructor_account_model";

type AccountSortKey = "id";
type AccountFilterKey = "state";
type AccountQuery = Pick<InstructorAccountBrowse, "query" | "state" | "sort">;

function stateLabel(state: InstructorAccountSummary["state"]): string {
  switch (state) {
    case "active":
      return "Active";
    case "deactivated":
      return "Deactivated";
    case "closed":
      return "Closed";
  }
}

function failureCopy(): string {
  return "That Instructor Account change could not be completed. Check the account state and try again.";
}

function accountAvatarMedia(account: InstructorAccountSummary): RecordContent["media"] {
  const avatar = PROVIDED_AVATAR_CATALOG.find((entry) => entry.id === account.providedAvatarId);
  return avatar === undefined ? undefined : { src: avatar.assetPath, alt: "" };
}

/** Sysadmin-only Account creation and lifecycle actions. */
export function InstructorAccountsPage(): JSX.Element {
  const runtime = useApplicationApi();
  const session = useSessionBootstrap();
  const accountFields: ReadonlyArray<
    RecordField<InstructorAccountSummary, AccountSortKey, AccountFilterKey>
  > = [
    {
      id: "account_id",
      label: "Account ID",
      role: "identity",
      sortKey: "id",
      value: (account) => account.id,
      defaultColumn: { priority: 0, width: "12rem" },
    },
    {
      id: "state",
      label: "State",
      filterKey: "state",
      value: (account) => account.state,
      defaultColumn: { priority: 1, width: "9rem" },
    },
    {
      id: "last_sign_in",
      label: "Last sign-in",
      value: (account) => account.lastSuccessfulSignIn,
      render: (account) =>
        formatSignInLabel(
          account.lastSuccessfulSignIn,
          createDisplayDateTimeFormatter(displayTimeZone()),
        ),
      defaultColumn: { priority: 2, width: "14rem" },
    },
    {
      id: "avatar",
      label: "Avatar",
      value: (account) => account.providedAvatarId,
      defaultColumn: { priority: 3, width: "8rem", visible: false },
    },
  ];
  const searchState = createSearchState<
    AccountQuery,
    InstructorAccountSummary,
    Pick<InstructorAccountList, "displayTimeZone" | "stateChoices">
  >(
    {
      initialQuery: { query: "", state: null, sort: "idAscending" },
      maximumPageSize: 100,
      cleanup: (query) => ({ ...query, query: query.query.trim() }),
      isEligible: () => true,
      getText: (query) => query.query,
      setText: (query, text) => ({ ...query, query: text }),
      rowId: (account) => account.id,
      content: accountContent,
      appliedFilters: (query) =>
        query.state === null
          ? []
          : [
              {
                id: "state",
                label: `State: ${stateLabel(query.state)}`,
                clear: (current): AccountQuery => ({ ...current, state: null }),
              },
            ],
      fetchPage: async (query, pageRequest) => {
        if (pageRequest.pageSize !== 50 && pageRequest.pageSize !== 100) {
          throw new Error("Instructor Account page size is unavailable");
        }
        const requestPageSize: 50 | 100 = pageRequest.pageSize === 50 ? 50 : 100;
        const current = session.state();
        if (current.kind !== "authenticated" || current.session.account.userRole !== "sysadmin") {
          throw new Error("Instructor Account access unavailable");
        }
        const page = await runtime.client.findInstructorAccounts({
          ...query,
          pageNumber: pageRequest.pageNumber,
          pageSize: requestPageSize,
        });
        return {
          items: page.items,
          pageNumber: page.pageNumber,
          pageSize: page.pageSize,
          totalCount: page.totalCount,
          filterCounts: {
            displayTimeZone: page.displayTimeZone,
            stateChoices: page.stateChoices,
          },
        };
      },
    },
    {},
  );
  let openedForSession = false;
  createEffect(() => {
    const current = session.state();
    if (
      !openedForSession &&
      current.kind === "authenticated" &&
      current.session.account.userRole === "sysadmin"
    ) {
      openedForSession = true;
      void searchState.open(searchState.query());
    }
  });
  const accountQuery: DataSheetQuery<AccountSortKey, AccountFilterKey> = {
    sort: {
      get value(): RecordSort<AccountSortKey> {
        return {
          key: "id",
          direction: searchState.query().sort === "idAscending" ? "ascending" : "descending",
        };
      },
      onChange: (sort) =>
        void searchState.sort((current) => ({
          ...current,
          sort: sort.direction === "ascending" ? "idAscending" : "idDescending",
        })),
    },
    filters: {
      get values() {
        return new Map<AccountFilterKey, ReadonlyArray<string>>([
          ["state", searchState.query().state === null ? [] : [searchState.query().state!]],
        ]);
      },
      onChange: (_field, values) =>
        void searchState.apply((current) => ({
          ...current,
          state: (values[0] as InstructorAccountState | undefined) ?? null,
        })),
      onClear: () => void searchState.apply((current) => ({ ...current, state: null })),
      choices: (_field, search, _cursor, signal) => {
        const list = stateChoices().filter((choice) => choice.key.includes(search.toLowerCase()));
        return Promise.resolve({
          choices: signal.aborted
            ? []
            : list.map((choice) => ({
                key: choice.key,
                label: stateLabel(choice.key),
                count: choice.count,
              })),
          selected: [],
          nextCursor: null,
        });
      },
    },
  };
  const [email, setEmail] = createSignal("");
  const [firstName, setFirstName] = createSignal("");
  const [lastName, setLastName] = createSignal("");
  const [affiliation, setAffiliation] = createSignal("");
  const [reasonByAccountId, setReasonByAccountId] = createSignal<Record<string, string>>({});
  const [busyAccountIds, setBusyAccountIds] = createSignal<ReadonlySet<string>>(new Set());
  const [creating, setCreating] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [announcement, setAnnouncement] = createSignal("");
  const [pendingDeactivation, setPendingDeactivation] =
    createSignal<InstructorAccountSummary | null>(null);
  const [editingDeactivation, setEditingDeactivation] =
    createSignal<InstructorAccountSummary | null>(null);

  function setReason(accountId: string, reason: string): void {
    setReasonByAccountId((current) => ({ ...current, [accountId]: reason }));
  }
  const stateChoices = (): ReadonlyArray<{ key: InstructorAccountState; count: number }> =>
    searchState.state().filterCounts?.stateChoices ?? [];
  const displayTimeZone = (): string => searchState.state().filterCounts?.displayTimeZone ?? "UTC";

  function setAccountBusy(accountId: string, busy: boolean): void {
    setBusyAccountIds((current) => {
      const next = new Set(current);
      if (busy) next.add(accountId);
      else next.delete(accountId);
      return next;
    });
  }

  function findAccounts(): void {
    const query = searchState.typedText().trim();
    if ([...query].length > 320 || /[\p{Cc}]/u.test(query)) {
      setError("Enter a name or email within 320 characters.");
      return;
    }
    setError(null);
    void searchState.apply((current) => ({ ...current, query }));
    setAnnouncement("Instructor Account list updated.");
  }

  async function createAccount(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (creating()) return;
    const normalizedEmail = email().trim().toLowerCase();
    if (normalizedEmail.length < 3 || normalizedEmail.length > 320) {
      setError("Enter a normalized Instructor Authentication Email within its allowed length.");
      return;
    }
    const setup = [firstName(), lastName(), affiliation()];
    if (
      setup.some((value) => !value.length || value !== value.trim() || /[\p{Cc}]/u.test(value)) ||
      [...firstName()].length > 100 ||
      [...lastName()].length > 100 ||
      [...affiliation()].length > 300
    ) {
      setError("Enter a trimmed first name, last name, and affiliation.");
      return;
    }
    setCreating(true);
    setError(null);
    try {
      const created = await runtime.client.createInstructorAccount({
        normalizedEmail,
        firstName: firstName(),
        lastName: lastName(),
        affiliation: affiliation(),
      });
      void searchState.refresh();
      setEmail("");
      setFirstName("");
      setLastName("");
      setAffiliation("");
      setAnnouncement(
        created.setupEmailSent
          ? "Instructor Account created and setup email sent."
          : "Instructor Account created, but setup email could not be sent.",
      );
    } catch {
      // ASVS 5.2.4: never reflect submitted email or a transport/server body.
      setError(
        "The Instructor Account could not be created. Check the normalized email and try again.",
      );
    } finally {
      setCreating(false);
    }
  }

  function requestDeactivation(account: InstructorAccountSummary): void {
    if (busyAccountIds().has(account.id)) return;
    const reason = reasonByAccountId()[account.id] ?? "";
    if (reason.length === 0 || reason.length > 1000 || reason !== reason.trim()) {
      setError("Enter a trimmed deactivation reason within 1,000 characters.");
      return;
    }
    setError(null);
    setEditingDeactivation(null);
    setPendingDeactivation(account);
  }

  async function deactivate(account: InstructorAccountSummary): Promise<void> {
    if (busyAccountIds().has(account.id)) return;
    const reason = reasonByAccountId()[account.id] ?? "";
    if (reason.length === 0 || reason.length > 1000 || reason !== reason.trim()) {
      setError("Enter a trimmed deactivation reason within 1,000 characters.");
      return;
    }
    setPendingDeactivation(null);
    setAccountBusy(account.id, true);
    setError(null);
    try {
      await runtime.client.deactivateInstructorAccount(account.id, { reason });
      void searchState.refresh();
      setReason(account.id, "");
      setAnnouncement(`Instructor Account ${account.id} deactivated.`);
    } catch {
      setError(failureCopy());
    } finally {
      setAccountBusy(account.id, false);
    }
  }

  async function reactivate(account: InstructorAccountSummary): Promise<void> {
    if (busyAccountIds().has(account.id)) return;
    setAccountBusy(account.id, true);
    setError(null);
    try {
      await runtime.client.reactivateInstructorAccount(account.id);
      void searchState.refresh();
      setAnnouncement(`Instructor Account ${account.id} reactivated.`);
    } catch {
      setError(failureCopy());
    } finally {
      setAccountBusy(account.id, false);
    }
  }

  async function resendSetupEmail(account: InstructorAccountSummary): Promise<void> {
    if (busyAccountIds().has(account.id)) return;
    setAccountBusy(account.id, true);
    setError(null);
    try {
      const sent = await runtime.client.sendInstructorSetupEmail(account.id);
      setAnnouncement(
        sent
          ? "Setup email sent."
          : "Setup email could not be sent. Check mail delivery settings and try again.",
      );
    } catch {
      setError("The setup email could not be sent. Check mail delivery settings and try again.");
    } finally {
      setAccountBusy(account.id, false);
    }
  }

  function accountContent(account: InstructorAccountSummary): RecordContent {
    const formatSignIn = createDisplayDateTimeFormatter(displayTimeZone());
    return {
      title: account.id,
      details: [
        { kind: "text", label: "State:", value: stateLabel(account.state) },
        {
          kind: "text",
          label: "Last successful sign-in:",
          value: formatSignInLabel(account.lastSuccessfulSignIn, formatSignIn),
        },
      ],
      media: accountAvatarMedia(account),
      actions:
        account.state === "active"
          ? [
              {
                id: "deactivate",
                kind: "command" as const,
                label: "Deactivate",
                disabled: busyAccountIds().has(account.id),
                onClick: (): void => {
                  setError(null);
                  setEditingDeactivation(account);
                },
              },
              {
                id: "resend-setup-email",
                kind: "command" as const,
                label: busyAccountIds().has(account.id)
                  ? "Sending setup email..."
                  : "Send setup email again",
                disabled: busyAccountIds().has(account.id),
                onClick: () => void resendSetupEmail(account),
              },
            ]
          : account.state === "deactivated"
            ? [
                {
                  id: "reactivate",
                  kind: "command" as const,
                  label: busyAccountIds().has(account.id)
                    ? "Updating..."
                    : "Reactivate Instructor Account",
                  primary: true,
                  disabled: busyAccountIds().has(account.id),
                  onClick: () => void reactivate(account),
                },
              ]
            : [],
    };
  }

  return (
    <PageFrame routeSurface="instructorAccounts">
      <p class="sr-only" role="status" aria-live="polite">
        {announcement()}
      </p>
      <SearchBlock
        layout="above"
        state={searchState}
        controls={
          <SearchControls
            state={searchState}
            textLabel="Name or email"
            maxlength={320}
            onSubmit={findAccounts}
            onClear={() => setError(null)}
          />
        }
        collection={{
          ariaLabel: "Instructor Accounts",
          fields: accountFields,
          query: accountQuery,
          emptyState: { title: "No Instructor Accounts match the current search or filters." },
        }}
      />
      <form
        class="auth-panel"
        aria-busy={creating()}
        novalidate
        onSubmit={(event) => void createAccount(event)}
      >
        <h2>Create Instructor Account</h2>
        <label for="instructor-account-email">
          Instructor Authentication Email
          <input
            id="instructor-account-email"
            name="normalizedEmail"
            type="email"
            value={email()}
            onInput={(event) => setEmail(event.currentTarget.value)}
            autocomplete="off"
            autocapitalize="none"
            spellcheck={false}
            maxlength={320}
            required
          />
        </label>
        <label for="instructor-account-first-name">
          First name
          <input
            id="instructor-account-first-name"
            name="firstName"
            type="text"
            value={firstName()}
            onInput={(event) => setFirstName(event.currentTarget.value)}
            autocomplete="off"
            maxlength={100}
            required
          />
        </label>
        <label for="instructor-account-last-name">
          Last name
          <input
            id="instructor-account-last-name"
            name="lastName"
            type="text"
            value={lastName()}
            onInput={(event) => setLastName(event.currentTarget.value)}
            autocomplete="off"
            maxlength={100}
            required
          />
        </label>
        <label for="instructor-account-affiliation">
          Affiliation
          <input
            id="instructor-account-affiliation"
            name="affiliation"
            type="text"
            value={affiliation()}
            onInput={(event) => setAffiliation(event.currentTarget.value)}
            autocomplete="organization"
            maxlength={300}
            required
          />
        </label>
        <button class="primary-action" type="submit" disabled={creating()}>
          {creating() ? "Creating Instructor Account..." : "Create Instructor Account"}
        </button>
      </form>
      <Show when={error()}>
        {(message) => (
          <section class="inline-error" role="alert">
            <p>{message()}</p>
          </section>
        )}
      </Show>
      <Show when={pendingDeactivation()}>
        {(account) => (
          <dialog
            class="confirmation-dialog"
            aria-labelledby="instructor-deactivation-heading"
            aria-describedby="instructor-deactivation-copy"
            ref={(element) => queueMicrotask(() => element.showModal())}
            onCancel={(event) => {
              event.preventDefault();
              setPendingDeactivation(null);
            }}
          >
            <h2 id="instructor-deactivation-heading">Deactivate this Instructor Account?</h2>
            <p id="instructor-deactivation-copy">
              This marks {account().id} deactivated, ends its current sessions, and blocks sign-in
              until a Sysadmin reactivates it. The deactivation reason stays with that state change.
            </p>
            <div class="action-row">
              <button
                ref={(element) => queueMicrotask(() => element.focus())}
                class="quiet-action"
                type="button"
                onClick={() => setPendingDeactivation(null)}
              >
                Keep this account active
              </button>
              <button
                class="primary-action"
                type="button"
                onClick={() => void deactivate(account())}
              >
                Deactivate Instructor Account
              </button>
            </div>
          </dialog>
        )}
      </Show>
      <Show when={editingDeactivation()}>
        {(account) => (
          <dialog
            class="confirmation-dialog"
            aria-labelledby="instructor-deactivation-editor-heading"
            aria-describedby="instructor-deactivation-editor-copy"
            ref={(element) => queueMicrotask(() => element.showModal())}
            onCancel={(event) => {
              event.preventDefault();
              setEditingDeactivation(null);
            }}
          >
            <h2 id="instructor-deactivation-editor-heading">Deactivate Instructor Account</h2>
            <p id="instructor-deactivation-editor-copy">
              Deactivation ends current sessions and blocks sign-in until reactivation. Enter the
              reason that stays with this account state change.
            </p>
            <label for={`deactivate-reason-${account().id}`}>
              Deactivation reason
              <input
                id={`deactivate-reason-${account().id}`}
                name={`deactivationReason-${account().id}`}
                type="text"
                value={reasonByAccountId()[account().id] ?? ""}
                onInput={(event) => setReason(account().id, event.currentTarget.value)}
                maxlength={1000}
                required
              />
            </label>
            <div class="action-row">
              <button
                class="quiet-action"
                type="button"
                onClick={() => setEditingDeactivation(null)}
              >
                Keep this account active
              </button>
              <button
                class="primary-action"
                type="button"
                disabled={busyAccountIds().has(account().id)}
                onClick={() => requestDeactivation(account())}
              >
                Continue to confirmation
              </button>
            </div>
          </dialog>
        )}
      </Show>
      <UnsavedChangesGuard
        dirty={() =>
          [email(), firstName(), lastName(), affiliation()].some((value) => value.length > 0) ||
          Object.values(reasonByAccountId()).some((reason) => reason.length > 0)
        }
        copy={{
          heading: "Discard Instructor Account changes?",
          description:
            "The new account details or deactivation reasons entered here have not been used.",
        }}
      />
    </PageFrame>
  );
}
