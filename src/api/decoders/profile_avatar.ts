import type {
  ProfileAvatar,
  ProfileAvatarView,
  ProfileImageCropInput,
  SelectProvidedProfileAvatarInput,
} from "../profile_avatar";
import { DecodeError, decodeRecord, decodeSafeInteger, decodeString, decodeUuid } from "../decoder";
import { field, requireOnlyFields } from "./shared";

const PROVIDED_AVATAR_ID = /^[a-z][a-z0-9-]{0,63}$/u;

/** Validates the same bounded integer geometry that the server applies to original bytes. */
export function decodeProfileImageCropInput(
  value: unknown,
  path = "request.crop",
): ProfileImageCropInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, [
    "sourceWidth",
    "sourceHeight",
    "horizontal",
    "vertical",
    "zoomPercent",
  ]);
  const sourceWidth = decodeSafeInteger(field(record, "sourceWidth", path), `${path}.sourceWidth`);
  const sourceHeight = decodeSafeInteger(
    field(record, "sourceHeight", path),
    `${path}.sourceHeight`,
  );
  const horizontal = decodeSafeInteger(field(record, "horizontal", path), `${path}.horizontal`);
  const vertical = decodeSafeInteger(field(record, "vertical", path), `${path}.vertical`);
  const zoomPercent = decodeSafeInteger(field(record, "zoomPercent", path), `${path}.zoomPercent`);
  if (sourceWidth < 128 || sourceHeight < 128 || sourceWidth * sourceHeight > 20_000_000) {
    throw new DecodeError(
      path,
      "source dimensions of at least 128 pixels and at most 20 million pixels",
    );
  }
  if (
    horizontal < 0 ||
    horizontal > 100 ||
    vertical < 0 ||
    vertical > 100 ||
    zoomPercent < 100 ||
    zoomPercent > 400
  ) {
    throw new DecodeError(path, "integer positions 0..100 and zoom 100..400 percent");
  }
  return { sourceWidth, sourceHeight, horizontal, vertical, zoomPercent };
}

function decodeProvidedAvatarId(value: unknown, path: string): string {
  const id = decodeString(value, path);
  if (!PROVIDED_AVATAR_ID.test(id)) throw new DecodeError(path, "a canonical provided avatar id");
  return id;
}

function decodeAvatar(value: unknown, path: string): ProfileAvatar {
  const record = decodeRecord(value, path);
  const kind = decodeString(field(record, "kind", path), `${path}.kind`);
  if (kind === "provided") {
    requireOnlyFields(record, path, ["kind", "providedAvatarId"]);
    return {
      kind,
      providedAvatarId: decodeProvidedAvatarId(
        field(record, "providedAvatarId", path),
        `${path}.providedAvatarId`,
      ),
    };
  }
  if (kind === "profileImage") {
    requireOnlyFields(record, path, ["kind", "profileImageId"]);
    return {
      kind,
      profileImageId: decodeUuid(field(record, "profileImageId", path), `${path}.profileImageId`),
    };
  }
  throw new DecodeError(`${path}.kind`, '"provided" or "profileImage"');
}

export function decodeProfileAvatarView(value: unknown, path = "response"): ProfileAvatarView {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["avatar"]);
  const avatar = field(record, "avatar", path);
  return { avatar: avatar === null ? null : decodeAvatar(avatar, `${path}.avatar`) };
}

export function decodeSelectProvidedProfileAvatarInput(
  value: unknown,
  path = "request",
): SelectProvidedProfileAvatarInput {
  const record = decodeRecord(value, path);
  requireOnlyFields(record, path, ["providedAvatarId"]);
  return {
    providedAvatarId: decodeProvidedAvatarId(
      field(record, "providedAvatarId", path),
      `${path}.providedAvatarId`,
    ),
  };
}
