// ribbon_account_avatar.tsx - role-neutral self-avatar projection for the persistent Ribbon.

import { Show, createEffect, createResource, createSignal, onCleanup, type JSX } from "solid-js";

import type { ApiClient } from "../../api/client";
import { RibbonIcon } from "../../ribbon/ribbon_icon";
import {
  PROFILE_AVATAR_CHANGED_EVENT,
  createProfileAvatarObjectUrlOwner,
  dispatchProfileAvatarChanged,
  observeProfileAvatarChanges,
  profileAvatarProjectionState,
} from "./profile_avatar_projection_state";

export { PROFILE_AVATAR_CHANGED_EVENT, dispatchProfileAvatarChanged };

type RibbonAccountAvatarClient = Pick<ApiClient, "fetchProfileAvatarImage" | "getProfileAvatar">;

/** Supplies the closed visual catalog when that catalog has an authoritative browser source. */
export type ProvidedAvatarRenderer = (providedAvatarId: string) => JSX.Element | undefined;

export interface RibbonAccountAvatarProps {
  readonly client: RibbonAccountAvatarClient;
  /**
   * Intentionally optional until the PLE-provided avatar catalog has one authoritative source.
   * A selected provided avatar is visibly pending rather than being represented as the generic icon.
   */
  readonly renderProvidedAvatar?: ProvidedAvatarRenderer;
}

/**
 * Projects the authenticated Account's selected avatar into the shared Ribbon control.
 * The self-only API remains the authority for both avatar state and protected image delivery.
 */
export function RibbonAccountAvatar(props: RibbonAccountAvatarProps): JSX.Element {
  const [avatar, { refetch }] = createResource(() => props.client.getProfileAvatar());
  const profileImageId = (): string | undefined => {
    const selected = avatar()?.avatar;
    return selected?.kind === "profileImage" ? selected.profileImageId : undefined;
  };
  const [image] = createResource(profileImageId, (reference) =>
    props.client.fetchProfileAvatarImage(reference),
  );
  const [imageUrl, setImageUrl] = createSignal<string>();
  const objectUrlOwner = createProfileAvatarObjectUrlOwner();

  function clearImageUrl(): void {
    setImageUrl(objectUrlOwner.replace(undefined));
  }

  createEffect(() => {
    if (image.loading || image.error !== undefined) {
      clearImageUrl();
      return;
    }
    const blob = image();
    if (blob === undefined) {
      clearImageUrl();
      return;
    }
    setImageUrl(objectUrlOwner.replace(blob));
  });

  const stopObservingAvatarChanges = observeProfileAvatarChanges(window, () => void refetch());
  onCleanup(() => {
    stopObservingAvatarChanges();
    objectUrlOwner.dispose();
  });

  const providedAvatar = (): JSX.Element | undefined => {
    const selected = avatar()?.avatar;
    if (selected?.kind !== "provided") return undefined;
    return props.renderProvidedAvatar?.(selected.providedAvatarId);
  };
  const selectedProvidedAvatarIsPending = (): boolean => {
    return (
      profileAvatarProjectionState(avatar()?.avatar, providedAvatar() !== undefined) ===
      "providedPending"
    );
  };

  return (
    <Show
      when={imageUrl()}
      fallback={
        <Show
          when={providedAvatar()}
          fallback={
            <Show
              when={selectedProvidedAvatarIsPending()}
              fallback={<RibbonIcon glyph="circle-user" />}
            >
              <span aria-label="Selected provided avatar is awaiting its catalog" role="img">
                ?
              </span>
            </Show>
          }
        >
          {(renderedAvatar) => renderedAvatar()}
        </Show>
      }
    >
      {(url) => <img src={url()} alt="" />}
    </Show>
  );
}
