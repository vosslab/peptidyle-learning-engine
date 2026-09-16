// profile_page.tsx - role-neutral authenticated-self Profile surface.

import { A } from "@solidjs/router";
import { Match, Show, Switch, createResource, createSignal, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
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
  const [profile] = createResource(() => applicationApi.client.getProfile());
  const [avatar, { mutate }] = createResource(() => applicationApi.client.getProfileAvatar());
  const [avatarSaving, setAvatarSaving] = createSignal(false);
  const [avatarMessage, setAvatarMessage] = createSignal("");
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
    <main class="page" data-route-surface="profile">
      <p class="eyebrow">{selfRoleLabel()} account</p>
      <h1>Your profile</h1>
      <p class="page-lede">This page shows settings for the Account you are signed in to.</p>
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
          <Match when={profile()}>{(settings) => <p>{settings().timeZone}</p>}</Match>
        </Switch>
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
    </main>
  );
}
