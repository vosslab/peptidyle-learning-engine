import type {
  ProfileAvatar,
  ProfileAvatarView,
  SelectProvidedProfileAvatarInput,
} from "../profile_avatar";
import { DecodeError, decodeRecord, decodeString, decodeUuid } from "../decoder";
import { field, requireOnlyFields } from "./shared";

const PROVIDED_AVATAR_ID = /^[a-z][a-z0-9-]{0,63}$/u;

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
