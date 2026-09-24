// profile_page.tsx - role-neutral authenticated-self Profile surface.

import { A } from "@solidjs/router";
import { For, Match, Show, Switch, createResource, createSignal, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import type { ProfileAvatarView } from "../api/profile_avatar";
import { useSessionBootstrap } from "../auth/session_context";
import { PROVIDED_AVATAR_CATALOG } from "../features/profile_avatar/avatar_catalog_generated";
import {
  AvatarVisual,
  ProvidedAvatarPicker,
} from "../features/profile_avatar/provided_avatar_picker";
import { dispatchProfileAvatarChanged } from "../features/profile_avatar/ribbon_account_avatar";
import { profileRoleMayManageImage } from "../features/profile_avatar/profile_avatar_role";
import { StaffAvatarSettings } from "../features/profile_avatar/staff_avatar_settings";

/**
 * The common Profile destination identifies the signed-in Account and owns
 * self-avatar selection. The server remains authoritative for both the
 * session role and the permissible avatar lifecycle.
 */
export function ProfilePage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const session = useSessionBootstrap();
  const [profile, { mutate: mutateProfile }] = createResource(() =>
    applicationApi.client.getProfile(),
  );
  const [avatar, { mutate }] = createResource(() => applicationApi.client.getProfileAvatar());
  const [avatarSaving, setAvatarSaving] = createSignal(false);
  const [avatarMessage, setAvatarMessage] = createSignal("");
  const [timeZoneDraft, setTimeZoneDraft] = createSignal<string>();
  const [timeZoneSaving, setTimeZoneSaving] = createSignal(false);
  const [timeZoneMessage, setTimeZoneMessage] = createSignal("");
  const selectedTimeZone = (): string => timeZoneDraft() ?? profile()?.timeZone ?? "UTC";
  const timeZones = (): readonly string[] => {
    const intl = Intl as typeof Intl & {
      readonly supportedValuesOf?: (key: "timeZone") => readonly string[];
    };
    const supported = intl.supportedValuesOf?.("timeZone") ?? [];
    return [...new Set([...supported, "UTC", selectedTimeZone()])].sort((left, right) =>
      left.localeCompare(right),
    );
  };
  const selfRoleLabel = (): string => {
    const state = session.state();
    if (state.kind !== "authenticated") return "Account";
    return (
      {
        student: "Student",
        instructor: "Instructor",
        sysadmin: "Sysadmin",
      } as const
    )[state.session.account.productRole];
  };
  const canManageProfileImage = (): boolean => {
    const state = session.state();
    return (
      state.kind === "authenticated" && profileRoleMayManageImage(state.session.account.productRole)
    );
  };
  const selectedProvidedAvatarId = (): string | undefined => {
    const selected = avatar()?.avatar;
    return selected?.kind === "provided" ? selected.providedAvatarId : undefined;
  };
  const hasRetiredProvidedAvatar = (): boolean => {
    const selected = selectedProvidedAvatarId();
    return (
      selected !== undefined &&
      PROVIDED_AVATAR_CATALOG.some((entry) => entry.id === selected && !entry.isSelectable)
    );
  };
  function recordAvatarChange(nextAvatar: ProfileAvatarView): void {
    mutate(nextAvatar);
    dispatchProfileAvatarChanged();
  }

  async function saveTimeZone(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (timeZoneSaving()) return;
    setTimeZoneSaving(true);
    setTimeZoneMessage("");
    try {
      const saved = await applicationApi.client.updateAccountSettings({
        timeZone: selectedTimeZone(),
      });
      mutateProfile(saved);
      setTimeZoneDraft(saved.timeZone);
      setTimeZoneMessage("Your time zone was saved.");
    } catch {
      setTimeZoneMessage("Your time zone could not be saved. Try again.");
    } finally {
      setTimeZoneSaving(false);
    }
  }

  async function selectProvidedAvatar(providedAvatarId: string): Promise<void> {
    if (avatarSaving() || selectedProvidedAvatarId() === providedAvatarId) return;
    setAvatarSaving(true);
    setAvatarMessage("");
    try {
      await applicationApi.client.selectProvidedProfileAvatar({ providedAvatarId });
      recordAvatarChange({ avatar: { kind: "provided", providedAvatarId } });
      setAvatarMessage("Your provided avatar was saved.");
    } catch {
      setAvatarMessage("Your provided avatar could not be saved. Try again.");
    } finally {
      setAvatarSaving(false);
    }
  }

  return (
    <PageFrame
      routeSurface="profile"
      eyebrow={`${selfRoleLabel()} account`}
      title="Your profile"
      lede="This page shows settings for the Account you are signed in to."
    >
      <section aria-labelledby="profile-time-zone-heading">
        <h2 id="profile-time-zone-heading">Time zone</h2>
        <Switch>
          <Match when={profile.loading}>
            <p class="calm-status" role="status">
              Loading your time zone...
            </p>
          </Match>
          <Match when={profile.error !== undefined}>
            <p role="alert">Your time zone is unavailable. Refresh to try again.</p>
          </Match>
          <Match when={profile()}>
            {(settings) => (
              <form aria-busy={timeZoneSaving()} onSubmit={(event) => void saveTimeZone(event)}>
                <label for="profile-time-zone">
                  Time zone
                  <select
                    id="profile-time-zone"
                    value={timeZoneDraft() ?? settings().timeZone}
                    disabled={timeZoneSaving()}
                    onInput={(event) => setTimeZoneDraft(event.currentTarget.value)}
                  >
                    <For each={timeZones()}>
                      {(timeZone) => <option value={timeZone}>{timeZone}</option>}
                    </For>
                  </select>
                </label>
                <button class="primary-action" type="submit" disabled={timeZoneSaving()}>
                  {timeZoneSaving() ? "Saving..." : "Save time zone"}
                </button>
              </form>
            )}
          </Match>
        </Switch>
        <Show when={timeZoneSaving() || timeZoneMessage()}>
          <p class="calm-status" role="status" aria-live="polite">
            {timeZoneSaving() ? "Saving your time zone..." : timeZoneMessage()}
          </p>
        </Show>
      </section>
      <section aria-labelledby="profile-avatar-heading">
        <h2 id="profile-avatar-heading">Avatar Gallery</h2>
        <Switch>
          <Match when={avatar.loading}>
            <p class="calm-status" role="status">
              Loading your avatar choices...
            </p>
          </Match>
          <Match when={avatar.error !== undefined}>
            <p role="alert">Your avatar choices are unavailable. Refresh to try again.</p>
          </Match>
          <Match when={avatar()}>
            <Show when={hasRetiredProvidedAvatar()}>
              <div aria-label="Your current provided avatar">
                <p>Your current provided avatar is no longer available for new selections.</p>
                <AvatarVisual avatarId={selectedProvidedAvatarId()!} size={64} />
              </div>
            </Show>
            <ProvidedAvatarPicker
              currentAvatarId={selectedProvidedAvatarId()}
              disabled={avatarSaving()}
              onSelect={(providedAvatarId) => void selectProvidedAvatar(providedAvatarId)}
            />
            <Show when={canManageProfileImage() && avatar()?.avatar?.kind === "profileImage"}>
              <p class="calm-status">
                Choosing a provided avatar removes your current profile image from this Account.
              </p>
            </Show>
            <Show when={avatarSaving() || avatarMessage()}>
              <p class="calm-status" role="status" aria-live="polite">
                {avatarSaving() ? "Saving your provided avatar..." : avatarMessage()}
              </p>
            </Show>
          </Match>
        </Switch>
      </section>
      <Show when={canManageProfileImage()}>
        <StaffAvatarSettings
          avatar={avatar}
          client={applicationApi.client}
          onAvatarChanged={recordAvatarChange}
        />
      </Show>
      <A class="quiet-link" href="/">
        Return to your dashboard
      </A>
    </PageFrame>
  );
}
