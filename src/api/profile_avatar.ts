/** The authenticated Account's one role-neutral avatar choice. */
export type ProfileAvatar =
  | {
      readonly kind: "provided";
      readonly providedAvatarId: string;
    }
  | {
      readonly kind: "profileImage";
      readonly profileImageId: string;
    };

/** Closed view returned by the self-only Account avatar routes. */
export interface ProfileAvatarView {
  readonly avatar: ProfileAvatar | null;
}

/** Closed request for selecting one PLE-provided avatar. */
export interface SelectProvidedProfileAvatarInput {
  readonly providedAvatarId: string;
}

/** Crop over the oriented original: integer positions 0..100, zoom 100..400 percent. */
export interface ProfileImageCropInput {
  readonly sourceWidth: number;
  readonly sourceHeight: number;
  readonly horizontal: number;
  readonly vertical: number;
  readonly zoomPercent: number;
}
