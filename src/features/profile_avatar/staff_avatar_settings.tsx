// staff_avatar_settings.tsx - self-only Profile-image capability for staff Account settings.

import {
  Show,
  createEffect,
  createResource,
  createSignal,
  onCleanup,
  type Accessor,
  type JSX,
} from "solid-js";

import type { ApiClient } from "../../api/client";
import type { ProfileAvatarView } from "../../api/profile_avatar";
import { RibbonIcon } from "../../ribbon/ribbon_icon";
import "./staff_avatar_settings.css";

type StaffAvatarClient = Pick<ApiClient, "fetchProfileAvatarImage" | "replaceProfileAvatarImage">;

export interface StaffAvatarSettingsProps {
  readonly client: StaffAvatarClient;
  /** The Profile page owns the one self-avatar resource shared with the picker. */
  readonly avatar: Accessor<ProfileAvatarView | undefined>;
  /** Commits a server-confirmed image mutation into that shared resource. */
  readonly onAvatarChanged: (avatar: ProfileAvatarView) => void;
}

/**
 * Presents the image-upload capability shared by Instructor and Sysadmin Accounts.
 *
 * A provided-avatar catalog is intentionally not synthesized here. Its closed visual
 * catalog will compose beside this capability once it has an authoritative source.
 */
export function StaffAvatarSettings(props: StaffAvatarSettingsProps): JSX.Element {
  const [selectedFile, setSelectedFile] = createSignal<File>();
  const [saving, setSaving] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [imageUrl, setImageUrl] = createSignal<string>();
  let input: HTMLInputElement | undefined;
  let activeUrl: string | undefined;

  const profileImageId = (): string | undefined => {
    const current = props.avatar();
    return current?.avatar?.kind === "profileImage" ? current.avatar.profileImageId : undefined;
  };
  const [image] = createResource(profileImageId, (reference) =>
    props.client.fetchProfileAvatarImage(reference),
  );

  function clearImageUrl(): void {
    if (activeUrl !== undefined) URL.revokeObjectURL(activeUrl);
    activeUrl = undefined;
    setImageUrl(undefined);
  }

  createEffect(() => {
    if (image.loading || image.error !== undefined || image() === undefined) {
      clearImageUrl();
      return;
    }
    clearImageUrl();
    activeUrl = URL.createObjectURL(image()!);
    setImageUrl(activeUrl);
  });
  onCleanup(clearImageUrl);

  async function replaceImage(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const file = selectedFile();
    if (file === undefined) {
      setMessage("Choose an image to replace your profile image.");
      return;
    }
    setSaving(true);
    setMessage("");
    try {
      props.onAvatarChanged(await props.client.replaceProfileAvatarImage(file));
      setSelectedFile(undefined);
      if (input !== undefined) input.value = "";
      setMessage("Your profile image was saved.");
    } catch {
      setMessage("Your profile image could not be saved. Try again.");
    } finally {
      setSaving(false);
    }
  }

  return (
    <section
      aria-labelledby="profile-image-heading"
      data-current-avatar-kind={props.avatar()?.avatar?.kind ?? "none"}
    >
      <h2 id="profile-image-heading">Profile image</h2>
      <div class="profile-thumbnail" aria-label="Profile image">
        <Show
          when={imageUrl()}
          fallback={
            <span class="profile-thumbnail-placeholder" aria-hidden="true">
              <RibbonIcon glyph="circle-user" />
            </span>
          }
        >
          {(url) => <img src={url()} alt="Your profile" />}
        </Show>
      </div>
      <Show when={image.error !== undefined}>
        <p class="calm-status" role="status">
          Your profile image is unavailable.
        </p>
      </Show>
      <form class="auth-panel" aria-busy={saving()} onSubmit={(event) => void replaceImage(event)}>
        <label for="profile-avatar-image">
          Replace profile image
          <input
            id="profile-avatar-image"
            type="file"
            accept="image/png,image/jpeg,image/webp"
            disabled={saving()}
            ref={(element) => {
              input = element;
            }}
            onChange={(event) => setSelectedFile(event.currentTarget.files?.[0])}
          />
        </label>
        <button class="primary-action" type="submit" disabled={saving()}>
          {saving() ? "Saving image..." : "Save profile image"}
        </button>
        <Show when={message()}>{(value) => <p role="status">{value()}</p>}</Show>
      </form>
    </section>
  );
}
