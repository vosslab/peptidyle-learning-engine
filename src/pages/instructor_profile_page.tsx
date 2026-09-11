import { For, Match, Show, Switch, createResource, createSignal, type JSX } from "solid-js";

import { useApplicationApi } from "../api/application_api";
import { ApiRequestError } from "../api/http_client";
import { ProfileThumbnail } from "../features/instructor_profile/profile_thumbnail";
import { dispatchProfileThumbnailReplaced } from "../features/instructor_profile/profile_thumbnail_url";

function availableTimeZones(current: string): readonly string[] {
  const intl = Intl as typeof Intl & {
    readonly supportedValuesOf?: (key: "timeZone") => readonly string[];
  };
  const supported = intl.supportedValuesOf?.("timeZone") ?? [];
  return [...new Set([...supported, "UTC", current])].sort((left, right) =>
    left.localeCompare(right),
  );
}

export function InstructorProfilePage(): JSX.Element {
  const applicationApi = useApplicationApi();
  const [profile, { mutate }] = createResource(() => applicationApi.client.getInstructorProfile());
  const [draft, setDraft] = createSignal<string>();
  const [saving, setSaving] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [thumbnail, { mutate: mutateThumbnail, refetch: refetchThumbnail }] = createResource(() =>
    applicationApi.client.getInstructorProfileThumbnail(),
  );
  const [selectedFile, setSelectedFile] = createSignal<File>();
  const [thumbnailSaving, setThumbnailSaving] = createSignal(false);
  const [thumbnailMessage, setThumbnailMessage] = createSignal("");
  let thumbnailInput: HTMLInputElement | undefined;
  const selected = (): string => draft() ?? profile()?.timeZone ?? "UTC";
  const thumbnailReference = (): string | null => {
    if (thumbnail.error !== undefined) return null;
    return thumbnail()?.reference ?? null;
  };

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    setSaving(true);
    setMessage("");
    try {
      const saved = await applicationApi.client.updateInstructorProfile({ timeZone: selected() });
      mutate(saved);
      setDraft(saved.timeZone);
      setMessage("Your time zone was saved.");
    } catch (error: unknown) {
      setMessage(
        error instanceof ApiRequestError
          ? "Your time zone could not be saved. Try again."
          : "Your time zone could not be saved.",
      );
    } finally {
      setSaving(false);
    }
  }

  async function replaceThumbnail(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const file = selectedFile();
    if (file === undefined) {
      setThumbnailMessage("Choose an image to replace your profile thumbnail.");
      return;
    }
    setThumbnailSaving(true);
    setThumbnailMessage("");
    try {
      const saved = await applicationApi.client.replaceInstructorProfileThumbnail(file);
      mutateThumbnail(saved);
      dispatchProfileThumbnailReplaced(saved);
      if (thumbnail.error !== undefined) void refetchThumbnail();
      setSelectedFile(undefined);
      if (thumbnailInput !== undefined) thumbnailInput.value = "";
      setThumbnailMessage("Your profile image was saved.");
    } catch (_error: unknown) {
      setThumbnailMessage("Your profile image could not be saved. Try again.");
    } finally {
      setThumbnailSaving(false);
    }
  }

  return (
    <main class="page" data-route-surface="instructorProfile">
      <p class="eyebrow">Instructor account</p>
      <h1>Profile</h1>
      <p class="page-lede">
        Choose the time zone used for your date and time entries and displays.
      </p>
      <p>
        Changing your time zone changes how existing deadlines are displayed. It does not move their
        stored instant. New date and time entries use your new time zone.
      </p>
      <section aria-labelledby="profile-image-heading">
        <h2 id="profile-image-heading">Profile image</h2>
        <ProfileThumbnail
          reference={thumbnailReference}
          unavailable={thumbnail.error !== undefined}
          client={applicationApi.client}
        />
        <form
          class="auth-panel"
          aria-busy={thumbnailSaving()}
          onSubmit={(event) => void replaceThumbnail(event)}
        >
          <label for="instructor-profile-thumbnail">
            Replace profile image
            <input
              id="instructor-profile-thumbnail"
              type="file"
              accept="image/png,image/jpeg,image/webp"
              disabled={thumbnailSaving()}
              ref={(element) => {
                thumbnailInput = element;
              }}
              onChange={(event) => setSelectedFile(event.currentTarget.files?.[0])}
            />
          </label>
          <button class="primary-action" type="submit" disabled={thumbnailSaving()}>
            {thumbnailSaving() ? "Saving image..." : "Save profile image"}
          </button>
          <Show when={thumbnailMessage()}>{(value) => <p role="status">{value()}</p>}</Show>
        </form>
      </section>
      <Switch>
        <Match when={profile.loading}>
          <p class="calm-status" role="status">
            Loading your profile...
          </p>
        </Match>
        <Match when={Boolean(profile.error)}>
          <p role="alert">Your profile is unavailable. Refresh to try again.</p>
        </Match>
        <Match when={profile()}>
          <form class="auth-panel" aria-busy={saving()} onSubmit={(event) => void save(event)}>
            <label for="instructor-profile-time-zone">
              Time zone
              <select
                id="instructor-profile-time-zone"
                value={selected()}
                disabled={saving()}
                onInput={(event) => setDraft(event.currentTarget.value)}
              >
                <For each={availableTimeZones(selected())}>
                  {(timeZone) => <option value={timeZone}>{timeZone}</option>}
                </For>
              </select>
            </label>
            <button class="primary-action" type="submit" disabled={saving()}>
              {saving() ? "Saving..." : "Save time zone"}
            </button>
            <Show when={message()}>
              {(value) => (
                <p role="status" aria-live="polite">
                  {value()}
                </p>
              )}
            </Show>
          </form>
        </Match>
      </Switch>
    </main>
  );
}
