// Live Sysadmin Instructor Account workflow; it never renders Authentication Email.

import { Show, createMemo, createResource, createSignal, type Accessor, type JSX } from "solid-js";

import type {
  InstructorAccountBrowse,
  InstructorAccountList,
  InstructorAccountPageSize,
  InstructorAccountState,
  InstructorAccountSummary,
} from "../api/instructor_account";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import { PageFrame } from "../components/page_frame";
import {
  RecordList,
  type RecordContent,
  type RecordListState,
} from "../components/record_list/record_list";
import { createDisplayDateTimeFormatter } from "../format_datetime";
import { PROVIDED_AVATAR_CATALOG } from "../features/profile_avatar/avatar_catalog_generated";
import { formatSignInLabel } from "./instructor_account_model";
import "./instructor_accounts_page.css";

const unavailableAccountList: InstructorAccountList = {
  accounts: [],
  displayTimeZone: "UTC",
  nextCursor: null,
};

function accountStateFilter(value: string): InstructorAccountState | null {
  if (value === "active" || value === "deactivated" || value === "closed") return value;
  return null;
}

function accountPageSize(value: string): InstructorAccountPageSize {
  if (value === "100") return 100;
  if (value === "250") return 250;
  return 50;
}

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
  const isSysadmin = createMemo(() => {
    const current = session.state();
    return current.kind === "authenticated" && current.session.account.userRole === "sysadmin";
  });
  const [draftQuery, setDraftQuery] = createSignal("");
  const [activeQuery, setActiveQuery] = createSignal("");
  const [stateFilter, setStateFilter] = createSignal<InstructorAccountState | null>(null);
  const [pageSize, setPageSize] = createSignal<InstructorAccountPageSize>(50);
  const [afterAccountId, setAfterAccountId] = createSignal<InstructorAccountSummary["id"] | null>(
    null,
  );
  const [cursorStack, setCursorStack] = createSignal<
    ReadonlyArray<InstructorAccountSummary["id"] | null>
  >([]);
  const [accounts, { refetch, mutate }] = createResource<
    InstructorAccountList,
    InstructorAccountBrowse | false
  >(
    () =>
      isSysadmin()
        ? {
            query: activeQuery(),
            state: stateFilter(),
            pageSize: pageSize(),
            afterAccountId: afterAccountId(),
          }
        : false,
    async (source) => {
      if (source === false) return unavailableAccountList;
      return runtime.client.findInstructorAccounts(source);
    },
  );
  const [email, setEmail] = createSignal("");
  const [verifiedInstructorDisplayName, setVerifiedInstructorDisplayName] = createSignal("");
  const [reasonByAccountId, setReasonByAccountId] = createSignal<Record<string, string>>({});
  const [busyAccountIds, setBusyAccountIds] = createSignal<ReadonlySet<string>>(new Set());
  const [creating, setCreating] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [announcement, setAnnouncement] = createSignal("");
  const [pendingDeactivation, setPendingDeactivation] =
    createSignal<InstructorAccountSummary | null>(null);

  const accountListState = (): RecordListState => {
    if (accounts.loading) return { kind: "loading", label: "Loading Instructor Accounts..." };
    if (accounts.error !== undefined) {
      return {
        kind: "error",
        title: "Instructor Accounts unavailable",
        message: "Instructor Accounts could not load. Check your connection and try again.",
        retry: () => void refetch(),
        retryLabel: "Retry",
      };
    }
    return { kind: "ready" };
  };

  function updateAccount(updated: InstructorAccountSummary): void {
    mutate((current) =>
      current === undefined
        ? current
        : {
            ...current,
            accounts: current.accounts.map((account) =>
              account.id === updated.id ? updated : account,
            ),
          },
    );
  }

  function setReason(accountId: string, reason: string): void {
    setReasonByAccountId((current) => ({ ...current, [accountId]: reason }));
  }

  function setAccountBusy(accountId: string, busy: boolean): void {
    setBusyAccountIds((current) => {
      const next = new Set(current);
      if (busy) next.add(accountId);
      else next.delete(accountId);
      return next;
    });
  }

  function resetPage(): void {
    setAfterAccountId(null);
    setCursorStack([]);
  }

  function findAccounts(event: SubmitEvent): void {
    event.preventDefault();
    const query = draftQuery().trim();
    if ([...query].length > 320 || /[\p{Cc}]/u.test(query)) {
      setError("Enter a name or email within 320 characters.");
      return;
    }
    setError(null);
    resetPage();
    setActiveQuery(query);
    setAnnouncement("Instructor Account list updated.");
  }

  function changeStateFilter(value: string): void {
    setError(null);
    resetPage();
    setStateFilter(accountStateFilter(value));
    setAnnouncement("Instructor Account list updated.");
  }

  function changePageSize(value: string): void {
    setError(null);
    resetPage();
    setPageSize(accountPageSize(value));
    setAnnouncement("Instructor Account list updated.");
  }

  function showNextPage(): void {
    const cursor = accounts()?.nextCursor ?? null;
    if (cursor === null || accounts.loading) return;
    setCursorStack((current) => [...current, afterAccountId()]);
    setAfterAccountId(cursor);
    setAnnouncement("Instructor Account list updated.");
  }

  function showPreviousPage(): void {
    const stack = cursorStack();
    const previous = stack[stack.length - 1];
    if (previous === undefined) return;
    setCursorStack(stack.slice(0, -1));
    setAfterAccountId(previous);
    setAnnouncement("Instructor Account list updated.");
  }

  async function createAccount(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (creating()) return;
    const normalizedEmail = email().trim().toLowerCase();
    const displayName = verifiedInstructorDisplayName().trim();
    if (normalizedEmail.length < 3 || normalizedEmail.length > 320) {
      setError("Enter a normalized Instructor Authentication Email within its allowed length.");
      return;
    }
    if (
      displayName.length === 0 ||
      [...displayName].length > 200 ||
      displayName !== verifiedInstructorDisplayName() ||
      /[\p{Cc}]/u.test(displayName)
    ) {
      setError(
        "Enter a trimmed, control-free verified Instructor display name within 200 characters.",
      );
      return;
    }
    setCreating(true);
    setError(null);
    try {
      const vetting = await runtime.client.completeInstructorIdentityVetting({
        normalizedEmail,
        verifiedInstructorDisplayName: displayName,
      });
      const created = await runtime.client.createInstructorAccount({
        normalizedEmail,
        vettingDecisionId: vetting.vettingDecisionId,
      });
      mutate((current) => {
        if (current === undefined) return current;
        const accounts = [created, ...current.accounts];
        return { ...current, accounts };
      });
      setEmail("");
      setVerifiedInstructorDisplayName("");
      setAnnouncement("Instructor Account created.");
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
      updateAccount(await runtime.client.deactivateInstructorAccount(account.id, { reason }));
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
      updateAccount(await runtime.client.reactivateInstructorAccount(account.id));
      setAnnouncement(`Instructor Account ${account.id} reactivated.`);
    } catch {
      setError(failureCopy());
    } finally {
      setAccountBusy(account.id, false);
    }
  }

  function accountContent(account: InstructorAccountSummary): RecordContent {
    const formatSignIn = createDisplayDateTimeFormatter(accounts()?.displayTimeZone ?? "UTC");
    return {
      title: account.id,
      details: [
        // ASVS 8.2.3 / 14.2.6: this list is Instructor Accounts only, so the role is Instructor.
        { kind: "text", label: "Role:", value: "Instructor" },
        { kind: "text", label: "State:", value: stateLabel(account.state) },
        {
          kind: "text",
          label: "Last successful sign-in:",
          value: formatSignInLabel(account.lastSuccessfulSignIn, formatSignIn),
        },
      ],
      media: accountAvatarMedia(account),
      actions:
        account.state === "deactivated"
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

  function renderAccountBody(account: Accessor<InstructorAccountSummary>): JSX.Element {
    return (
      <>
        <Show when={account().state === "active"}>
          <section
            class="instructor-account-consequence"
            aria-label="Deactivate Instructor Account"
          >
            <p>Deactivation ends current sessions and blocks sign-in until reactivation.</p>
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
            <button
              class="quiet-action"
              type="button"
              disabled={busyAccountIds().has(account().id)}
              onClick={() => requestDeactivation(account())}
            >
              {busyAccountIds().has(account().id) ? "Updating..." : "Deactivate Instructor Account"}
            </button>
          </section>
        </Show>
        <Show when={account().state === "closed"}>
          <p>This Instructor Account is closed and cannot be changed here.</p>
        </Show>
      </>
    );
  }

  return (
    <PageFrame
      routeSurface="instructorAccounts"
      headingId="instructor-accounts-heading"
      eyebrow="System administration"
      title="Instructor Accounts"
      lede="Create and manage Instructor Account access. Each row shows the Account ID, Instructor role, account state, and last successful sign-in. Email and display name stay off this list."
    >
      <p class="sr-only" role="status" aria-live="polite">
        {announcement()}
      </p>
      <form class="auth-panel" novalidate onSubmit={findAccounts}>
        <h2>Find an Instructor Account</h2>
        <label for="instructor-account-find">
          Name or email
          <input
            id="instructor-account-find"
            name="instructorAccountFind"
            type="search"
            value={draftQuery()}
            onInput={(event) => setDraftQuery(event.currentTarget.value)}
            autocomplete="off"
            autocapitalize="none"
            spellcheck={false}
            maxlength={320}
          />
        </label>
        <label for="instructor-account-state">
          Account state
          <select
            id="instructor-account-state"
            name="accountState"
            value={stateFilter() ?? ""}
            onChange={(event) => changeStateFilter(event.currentTarget.value)}
          >
            <option value="">Any state</option>
            <option value="active">Active</option>
            <option value="deactivated">Deactivated</option>
            <option value="closed">Closed</option>
          </select>
        </label>
        <label for="instructor-account-page-size">
          Accounts per page
          <select
            id="instructor-account-page-size"
            name="accountPageSize"
            value={String(pageSize())}
            onChange={(event) => changePageSize(event.currentTarget.value)}
          >
            <option value="50">50</option>
            <option value="100">100</option>
            <option value="250">250</option>
          </select>
        </label>
        <button class="primary-action" type="submit">
          Find Instructor Account
        </button>
      </form>
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
        <label for="instructor-account-verified-display-name">
          Verified Instructor Display Name
          <input
            id="instructor-account-verified-display-name"
            name="verifiedInstructorDisplayName"
            type="text"
            value={verifiedInstructorDisplayName()}
            onInput={(event) => setVerifiedInstructorDisplayName(event.currentTarget.value)}
            autocomplete="off"
            maxlength={200}
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
      <div class="action-row">
        <button
          class="quiet-action"
          type="button"
          disabled={cursorStack().length === 0 || accounts.loading}
          onClick={showPreviousPage}
        >
          Previous Instructor Accounts
        </button>
        <button
          class="quiet-action"
          type="button"
          disabled={accounts.loading || (accounts()?.nextCursor ?? null) === null}
          onClick={showNextPage}
        >
          Next Instructor Accounts
        </button>
      </div>
      <RecordList
        ariaLabel="Instructor Accounts"
        emptyState={{
          title:
            activeQuery().length > 0
              ? "No Instructor Account matches that name or email."
              : stateFilter() === null
                ? "No Instructor Accounts are available."
                : "No Instructor Account matches that state.",
        }}
        recordId={(account) => account.id}
        content={accountContent}
        renderBody={renderAccountBody}
        rows={accounts()?.accounts ?? []}
        state={accountListState()}
      />
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
                class="quiet-action"
                type="button"
                onClick={() => setPendingDeactivation(null)}
              >
                Keep this account active
              </button>
              <button
                ref={(element) => queueMicrotask(() => element.focus())}
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
    </PageFrame>
  );
}
