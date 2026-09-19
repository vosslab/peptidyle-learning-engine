// Live Sysadmin Instructor Account workflow; it never renders Authentication Email.

import { For, Show, createMemo, createResource, createSignal, type JSX } from "solid-js";

import type { InstructorAccountList, InstructorAccountSummary } from "../api/instructor_account";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import { AvatarVisual } from "../features/profile_avatar/provided_avatar_picker";
import { RibbonIcon } from "../ribbon/ribbon_icon";
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
  const [busyAccountId, setBusyAccountId] = createSignal<string | null>(null);
  const [creating, setCreating] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [announcement, setAnnouncement] = createSignal("");

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
    const reason = reasonByAccountId()[account.id] ?? "";
    if (reason.length === 0 || reason.length > 1000 || reason !== reason.trim()) {
      setError("Enter a trimmed deactivation reason within 1,000 characters.");
      return;
    }
    setBusyAccountId(account.id);
    setError(null);
    try {
      updateAccount(await runtime.client.deactivateInstructorAccount(account.id, { reason }));
      setReason(account.id, "");
      setAnnouncement(`Instructor Account ${account.id} deactivated.`);
    } catch {
      setError(failureCopy());
    } finally {
      setBusyAccountId(null);
    }
  }

  async function reactivate(account: InstructorAccountSummary): Promise<void> {
    setBusyAccountId(account.id);
    setError(null);
    try {
      updateAccount(await runtime.client.reactivateInstructorAccount(account.id));
      setAnnouncement(`Instructor Account ${account.id} reactivated.`);
    } catch {
      setError(failureCopy());
    } finally {
      setBusyAccountId(null);
    }
  }

  return (
    <section
      class="page"
      data-route-surface="instructorAccounts"
      aria-labelledby="instructor-accounts-heading"
    >
      <p class="eyebrow">System administration</p>
      <h1 id="instructor-accounts-heading">Instructor Accounts</h1>
      <p class="page-lede">
        Create and manage Instructor Account access. This workspace intentionally lists only an
        Account ID, state, and last successful sign-in.
      </p>
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
            maxlength={400}
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
      <Show when={accounts.loading}>
        <p class="loading-state">Loading Instructor Accounts...</p>
      </Show>
      <Show when={accounts.error !== undefined}>
        <section class="inline-error" role="alert">
          <p>Instructor Accounts could not load. Check your connection and try again.</p>
          <button class="quiet-action" type="button" onClick={() => void refetch()}>
            Retry
          </button>
        </section>
      </Show>
      <Show when={accounts() !== undefined && accounts.error === undefined}>
        <Show when={accounts()} keyed>
          {(list) => (
            <section aria-label="Instructor Accounts">
              <p class="page-lede">
                Last successful sign-in times use your time zone: {list.displayTimeZone}.
              </p>
              <For
                each={list.accounts}
                fallback={<p class="empty-state">No Instructor Accounts are available.</p>}
              >
                {(account) => (
                  <article class="auth-panel">
                    <h2>
                      <Show
                        when={account.providedAvatarId}
                        fallback={<RibbonIcon glyph="circle-user" />}
                      >
                        {(providedAvatarId) => (
                          <AvatarVisual avatarId={providedAvatarId()} decorative size={24} />
                        )}
                      </Show>{" "}
                      {account.id}
                    </h2>
                    <p>State: {stateLabel(account.state)}</p>
                    <p>
                      Last successful sign-in:{" "}
                      {formatSignInLabel(account.lastSuccessfulSignIn, list.displayTimeZone)}
                    </p>
                    <Show when={account.state === "active"}>
                      <label for={`deactivate-reason-${account.id}`}>
                        Deactivation reason
                        <input
                          id={`deactivate-reason-${account.id}`}
                          name={`deactivationReason-${account.id}`}
                          type="text"
                          value={reasonByAccountId()[account.id] ?? ""}
                          onInput={(event) => setReason(account.id, event.currentTarget.value)}
                          maxlength={1000}
                          required
                        />
                      </label>
                      <button
                        class="quiet-action"
                        type="button"
                        disabled={busyAccountId() === account.id}
                        onClick={() => void deactivate(account)}
                      >
                        {busyAccountId() === account.id
                          ? "Updating..."
                          : "Deactivate Instructor Account"}
                      </button>
                    </Show>
                    <Show when={account.state === "deactivated"}>
                      <button
                        class="primary-action"
                        type="button"
                        disabled={busyAccountId() === account.id}
                        onClick={() => void reactivate(account)}
                      >
                        {busyAccountId() === account.id
                          ? "Updating..."
                          : "Reactivate Instructor Account"}
                      </button>
                    </Show>
                    <Show when={account.state === "closed"}>
                      <p>This Instructor Account is closed and cannot be changed here.</p>
                    </Show>
                  </article>
                )}
              </For>
            </section>
          )}
        </Show>
      </Show>
    </section>
  );
}
