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
import type { ProfileAvatarView, ProfileImageCropInput } from "../../api/profile_avatar";
import { RibbonIcon } from "../../ribbon/ribbon_icon";
import {
  PROFILE_IMAGE_SIDE_PIXELS,
  decodeProfileImage,
  drawProfileImageCrop,
  profileImageCrop,
} from "./profile_image_crop";
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
  const [source, setSource] = createSignal<ImageBitmap>();
  const [preview, setPreview] = createSignal<HTMLCanvasElement>();
  const [loading, setLoading] = createSignal(false);
  const [horizontal, setHorizontal] = createSignal(50);
  const [vertical, setVertical] = createSignal(50);
  const [zoom, setZoom] = createSignal(100);
  const [saving, setSaving] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [imageUrl, setImageUrl] = createSignal<string>();
  let input: HTMLInputElement | undefined;
  let activeUrl: string | undefined;
  let selection = 0;
  let original: File | undefined;

  const cropInput = (bitmap: ImageBitmap): ProfileImageCropInput => ({
    sourceWidth: bitmap.width,
    sourceHeight: bitmap.height,
    horizontal: horizontal(),
    vertical: vertical(),
    zoomPercent: zoom(),
  });

  const profileImageId = (): string | undefined => {
    const current = props.avatar();
    return current?.avatar?.kind === "profileImage" ? current.avatar.profileImageId : undefined;
  };
  const [image] = createResource(profileImageId, (id) => props.client.fetchProfileAvatarImage(id));

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
  onCleanup(() => {
    selection += 1;
    source()?.close();
  });

  function resetCrop(): void {
    setHorizontal(50);
    setVertical(50);
    setZoom(100);
  }

  async function selectImage(file: File | undefined): Promise<void> {
    const currentSelection = ++selection;
    source()?.close();
    setSource(undefined);
    original = undefined;
    setMessage("");
    setLoading(file !== undefined);
    resetCrop();
    if (file === undefined) return;
    try {
      const bitmap = await decodeProfileImage(file);
      if (currentSelection !== selection) {
        bitmap.close();
        return;
      }
      original = file;
      setSource(bitmap);
    } catch (error) {
      if (currentSelection === selection) {
        setMessage(error instanceof Error ? error.message : "Your image could not be opened.");
      }
    } finally {
      if (currentSelection === selection) setLoading(false);
    }
  }

  createEffect(() => {
    const bitmap = source();
    const canvas = preview();
    if (bitmap === undefined || canvas === undefined) return;
    try {
      drawProfileImageCrop(canvas, bitmap, profileImageCrop(cropInput(bitmap)));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Your image preview is unavailable.");
      setSource(undefined);
      bitmap.close();
    }
  });

  async function replaceImage(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (saving()) return;
    const canvas = preview();
    const bitmap = source();
    if (loading() || bitmap === undefined || original === undefined || canvas === undefined) {
      setMessage("Choose an image to replace your profile image.");
      return;
    }
    setSaving(true);
    setMessage("");
    try {
      props.onAvatarChanged(
        await props.client.replaceProfileAvatarImage(original, cropInput(bitmap)),
      );
      source()?.close();
      setSource(undefined);
      original = undefined;
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
        <p id="profile-image-help">
          Choose a PNG, JPEG, or WebP at least 128 pixels wide and tall, up to 8 MiB and 20 million
          pixels. Position and crop it in the square preview before saving.
        </p>
        <label class="profile-image-file-picker" for="profile-avatar-image">
          Replace profile image
          <input
            id="profile-avatar-image"
            type="file"
            accept="image/png,image/jpeg,image/webp"
            aria-describedby="profile-image-help"
            disabled={saving()}
            ref={(element) => {
              input = element;
            }}
            onChange={(event) => void selectImage(event.currentTarget.files?.[0])}
          />
        </label>
        <Show when={loading()}>
          <p role="status">Preparing your image...</p>
        </Show>
        <Show when={source()}>
          <fieldset class="profile-image-crop" disabled={saving()}>
            <legend>Position and crop</legend>
            <canvas
              class="profile-image-crop-preview"
              width={PROFILE_IMAGE_SIDE_PIXELS}
              height={PROFILE_IMAGE_SIDE_PIXELS}
              ref={setPreview}
              role="img"
              aria-label="Square profile image preview"
            />
            <div class="profile-image-crop-controls">
              <label for="profile-image-horizontal">
                Horizontal position
                <input
                  id="profile-image-horizontal"
                  type="range"
                  min="0"
                  max="100"
                  step="1"
                  value={horizontal()}
                  onInput={(event) => setHorizontal(event.currentTarget.valueAsNumber)}
                />
              </label>
              <label for="profile-image-vertical">
                Vertical position
                <input
                  id="profile-image-vertical"
                  type="range"
                  min="0"
                  max="100"
                  step="1"
                  value={vertical()}
                  onInput={(event) => setVertical(event.currentTarget.valueAsNumber)}
                />
              </label>
              <label for="profile-image-zoom">
                Zoom
                <input
                  id="profile-image-zoom"
                  type="range"
                  min="100"
                  max="400"
                  step="5"
                  value={zoom()}
                  onInput={(event) => setZoom(event.currentTarget.valueAsNumber)}
                />
              </label>
              <button type="button" class="secondary-action" onClick={resetCrop}>
                Reset crop
              </button>
            </div>
          </fieldset>
        </Show>
        <button
          class="primary-action"
          type="submit"
          disabled={saving() || loading() || source() === undefined}
        >
          {saving() ? "Saving image..." : "Save profile image"}
        </button>
        <Show when={message()}>{(value) => <p role="status">{value()}</p>}</Show>
      </form>
    </section>
  );
}
