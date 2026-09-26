// Live Sysadmin Instructor Account workflow; it never renders Authentication Email.

import { Show, createMemo, createResource, createSignal, type Accessor, type JSX } from "solid-js";

import type { InstructorAccountList, InstructorAccountSummary } from "../api/instructor_account";
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

const unavailableAccountList: InstructorAccountList = {
  accounts: [],
  displayTimeZone: "UTC",
};

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
    return current.kind === "authenticated" && current.session.account.productRole === "sysadmin";
  });
  const [accounts, { refetch, mutate }] = createResource<InstructorAccountList, boolean>(
    isSysadmin,
    async (allowed) => (allowed ? runtime.client.listInstructorAccounts() : unavailableAccountList),
  );
  const [email, setEmail] = createSignal("");
  const [verifiedInstructorDisplayName, setVerifiedInstructorDisplayName] = createSignal("");
  const [reasonByAccountId, setReasonByAccountId] = createSignal<Record<string, string>>({});
  const [busyAccountIds, setBusyAccountIds] = createSignal<ReadonlySet<string>>(new Set());
  const [creating, setCreating] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [announcement, setAnnouncement] = createSignal("");

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

  async function deactivate(account: InstructorAccountSummary): Promise<void> {
    if (busyAccountIds().has(account.id)) return;
    const reason = reasonByAccountId()[account.id] ?? "";
    if (reason.length === 0 || reason.length > 1000 || reason !== reason.trim()) {
      setError("Enter a trimmed deactivation reason within 1,000 characters.");
      return;
    }
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
                label: busyAccountIds().has(account.id)
                  ? "Updating..."
                  : "Deactivate Instructor Account",
                disabled: busyAccountIds().has(account.id),
                onClick: () => void deactivate(account),
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

  function renderAccountBody(account: Accessor<InstructorAccountSummary>): JSX.Element {
    return (
      <div class="auth-panel">
        <Show when={account().state === "active"}>
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
        </Show>
        <Show when={account().state === "closed"}>
          <p>This Instructor Account is closed and cannot be changed here.</p>
        </Show>
      </div>
    );
  }

  return (
    <PageFrame
      routeSurface="instructorAccounts"
      headingId="instructor-accounts-heading"
      eyebrow="System administration"
      title="Instructor Accounts"
      lede="Create and manage Instructor Account access. This workspace intentionally lists only an Account ID, state, and last successful sign-in."
    >
      <p class="sr-only" role="status" aria-live="polite">
        {announcement()}
      </p>
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
      <RecordList
        ariaLabel="Instructor Accounts"
        emptyState={{ title: "No Instructor Accounts are available." }}
        recordId={(account) => account.id}
        content={accountContent}
        renderBody={renderAccountBody}
        rows={accounts()?.accounts ?? []}
        state={accountListState()}
      />
    </PageFrame>
  );
}
