// instructor_profile_link.tsx - public-within-PLE Instructor identity link and avatar.

import { Show, createEffect, createResource, createSignal, onCleanup, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import type { ProfileAvatar } from "../api/profile_avatar";
import { AvatarVisual } from "../features/profile_avatar/provided_avatar_picker";
import {
  fetchSharedProfileAvatarImage,
  getSharedInstructorProfile,
} from "./instructor_profile_requests";
import "./instructor_profile_link.css";

function InstructorAvatar(props: {
  readonly avatar: ProfileAvatar | null | undefined;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const profileImageId = (): string | undefined =>
    props.avatar?.kind === "profileImage" ? props.avatar.profileImageId : undefined;
  const [image] = createResource(profileImageId, (id) =>
    fetchSharedProfileAvatarImage(applicationApi.client, id),
  );
  const [imageUrl, setImageUrl] = createSignal<string>();
  let activeUrl: string | undefined;

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

  return (
    <span class="instructor-profile-avatar">
      <Show
        when={imageUrl()}
        fallback={
          <Show
            when={props.avatar?.kind === "provided" ? props.avatar.providedAvatarId : undefined}
            fallback={
              <span aria-label="Instructor avatar unavailable" role="img">
                ?
              </span>
            }
          >
            {(avatarId) => <AvatarVisual avatarId={avatarId()} decorative size={24} />}
          </Show>
        }
      >
        {(url) => <img alt="" src={url()} />}
      </Show>
    </span>
  );
}

/** Links a displayed Instructor name and avatar to the signed-in Profile surface. */
export function InstructorProfileLink(props: {
  readonly accountId: string;
  readonly displayName: string;
  /** Callers with a Profile projection avoid a duplicate read. */
  readonly avatar?: ProfileAvatar | null;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const [profile] = createResource(
    () => (props.avatar === undefined ? props.accountId : undefined),
    (accountId) => getSharedInstructorProfile(applicationApi.client, accountId),
  );
  const avatar = (): ProfileAvatar | null | undefined => {
    if (props.avatar !== undefined) return props.avatar;
    if (profile.error !== undefined) return undefined;
    return profile()?.avatar;
  };
  return (
    <a
      class="instructor-profile-link"
      href={`/instructors/${encodeURIComponent(props.accountId)}`}
      target="_blank"
      rel="noopener"
    >
      <InstructorAvatar avatar={avatar()} />
      {props.displayName}
    </a>
  );
}
