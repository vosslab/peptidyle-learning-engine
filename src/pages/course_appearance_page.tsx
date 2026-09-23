// course_appearance_page.tsx - Instructor Course Appearance controls.

import { For, Show, createEffect, createMemo, createSignal, onCleanup, type JSX } from "solid-js";

import type { CourseAppearanceView } from "../../generated/api/CourseAppearanceView";
import type { CourseInstanceId } from "../../generated/api/CourseInstanceId";
import type { CourseBannerAlternativeText } from "../../generated/api/CourseBannerAlternativeText";
import type { CourseTheme } from "../../generated/api/CourseTheme";
import { useApplicationApi } from "../api/application_api";
import { PageFrame } from "../components/page_frame";
import {
  courseRouteView,
  useCourseThemePresentation,
} from "../features/course_appearance/course_theme_context";
import { COURSE_APPEARANCE_STYLES } from "../features/course_appearance/course_appearance_styles";
import {
  COURSE_THEME_OPTIONS,
  courseThemeStyle,
} from "../features/course_appearance/course_theme_registry";
import {
  useReplaceCourseAppearance,
  useRetryRouteScope,
  useRouteScopeData,
  useRouteScopeLoadState,
} from "../ribbon/route_scope_context";

type SaveState = "ready" | "saving" | "error" | "saved";
type AlternativeTextChoice = "decorative" | "informative";

// HG requires one small, centered 5:1 Course banner.  This follows the Course
// entry presentation while the server rendition migration remains separate.
const COURSE_BANNER_5_TO_1_STYLES = `
.course-appearance-banner-preview-group {
  max-inline-size: 64rem;
}

.course-appearance-banner-presentation {
  box-sizing: border-box;
  display: block;
  overflow: hidden;
  aspect-ratio: 5 / 1;
  border: 1px solid var(--ple-border);
  border-radius: var(--ple-radius-control, 0.25rem);
}

.course-appearance-banner-presentation .course-appearance-banner-image {
  inline-size: 100%;
  block-size: 100%;
  border: 0;
  border-radius: 0;
}
`;

function ThemePalettePreview(props: { readonly theme: CourseTheme }): JSX.Element {
  const option = COURSE_THEME_OPTIONS.find((candidate) => candidate.id === props.theme);
  return (
    <div
      class="course-appearance-palette-preview"
      aria-label={`${option?.tokens.name ?? props.theme} palette roles`}
    >
      <span class="course-appearance-palette-canvas">Canvas</span>
      <span class="course-appearance-palette-secondary">Secondary</span>
      <span class="course-appearance-palette-accent">Accent</span>
    </div>
  );
}

function AppearanceThemeEditor(props: {
  readonly courseInstanceId: CourseInstanceId;
  readonly storedAppearance: () => CourseAppearanceView;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const replaceCourseAppearance = useReplaceCourseAppearance();
  const presentAppearance = useCourseThemePresentation();
  const storedTheme = createMemo(() => props.storedAppearance().theme);
  const [selectedTheme, setSelectedTheme] = createSignal<CourseTheme>(storedTheme());
  const [themeDraftDirty, setThemeDraftDirty] = createSignal(false);
  const [saveState, setSaveState] = createSignal<SaveState>("ready");
  const [message, setMessage] = createSignal("");
  const saving = createMemo(() => saveState() === "saving");

  // A banner write replaces the aggregate cache. Do not replace a live theme draft with its
  // unchanged stored theme while that independent banner mutation is completing.
  createEffect(() => {
    const theme = storedTheme();
    if (!themeDraftDirty()) setSelectedTheme(theme);
  });
  onCleanup(() => presentAppearance?.(undefined));
  function selectTheme(theme: CourseTheme): void {
    if (saving()) return;
    setSelectedTheme(theme);
    setThemeDraftDirty(true);
    setSaveState("ready");
    setMessage("");
    presentAppearance?.({ ...props.storedAppearance(), theme });
  }
  async function saveTheme(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (saving()) return;
    setSaveState("saving");
    setMessage("");
    try {
      const saved = await applicationApi.client.updateCourseTheme(props.courseInstanceId, {
        theme: selectedTheme(),
      });
      replaceCourseAppearance(props.courseInstanceId, saved);
      presentAppearance?.(undefined);
      setSelectedTheme(saved.theme);
      setThemeDraftDirty(false);
      setSaveState("saved");
      setMessage("Theme saved.");
    } catch {
      presentAppearance?.(undefined);
      setSelectedTheme(storedTheme());
      setThemeDraftDirty(false);
      setSaveState("error");
      setMessage("Theme could not save. The saved theme is still displayed.");
    }
  }
  return (
    <form class="course-appearance-form" onSubmit={(event) => void saveTheme(event)}>
      <section class="course-appearance-section" aria-labelledby="course-theme-heading">
        <fieldset class="course-appearance-fieldset" disabled={saving()}>
          <legend id="course-theme-heading">Theme</legend>
          <p class="course-appearance-help">
            Selecting a theme previews it immediately. Save to make it visible to Course members.
          </p>
          <div class="course-appearance-theme-grid">
            <For each={COURSE_THEME_OPTIONS}>
              {(option) => (
                <label
                  class="course-appearance-theme-card"
                  data-course-theme-option={option.id}
                  style={courseThemeStyle(option.tokens)}
                >
                  <input
                    type="radio"
                    name="course-theme"
                    value={option.id}
                    checked={selectedTheme() === option.id}
                    onInput={() => selectTheme(option.id)}
                  />
                  <span class="course-appearance-theme-label">{option.tokens.name}</span>
                  <ThemePalettePreview theme={option.id} />
                </label>
              )}
            </For>
          </div>
        </fieldset>
        <div class="course-appearance-save-actions">
          <button type="submit" disabled={saving()}>
            {saving() ? "Saving theme..." : "Save theme"}
          </button>
          <Show when={message()}>
            <p
              class={
                saveState() === "error" ? "course-appearance-error" : "course-appearance-save-note"
              }
              role={saveState() === "error" ? "alert" : "status"}
            >
              {message()}
            </p>
          </Show>
        </div>
      </section>
    </form>
  );
}

function BannerImage(props: {
  readonly url: string;
  readonly alternativeText: CourseBannerAlternativeText;
}): JSX.Element {
  const alternativeText = props.alternativeText;
  const decorative = alternativeText.kind === "decorative";
  return (
    <div class="course-appearance-banner-presentation">
      <img
        class="course-appearance-banner-image"
        src={props.url}
        alt={decorative ? "" : alternativeText.text}
        aria-hidden={decorative || undefined}
      />
    </div>
  );
}

function AppearanceBannerEditor(props: {
  readonly courseInstanceId: CourseInstanceId;
  readonly courseLongName: string;
  readonly storedAppearance: () => CourseAppearanceView;
}): JSX.Element {
  const applicationApi = useApplicationApi();
  const replaceCourseAppearance = useReplaceCourseAppearance();
  const [file, setFile] = createSignal<File>();
  const [localUrl, setLocalUrl] = createSignal<string>();
  const [bannerUrl, setBannerUrl] = createSignal<string>();
  let savedBannerUrl: string | undefined;
  let fileInput: HTMLInputElement | undefined;
  const [alternativeTextChoice, setAlternativeTextChoice] =
    createSignal<AlternativeTextChoice>("decorative");
  const [alternativeText, setAlternativeText] = createSignal("");
  const [saveState, setSaveState] = createSignal<SaveState>("ready");
  const [message, setMessage] = createSignal("");
  const saving = createMemo(() => saveState() === "saving");
  const savedBanner = createMemo(() => props.storedAppearance().banner);
  const savedBannerId = createMemo(() => props.storedAppearance().banner?.id);
  const usableAlternativeText = createMemo(() => alternativeText().trim());
  const canSave = createMemo(
    () =>
      file() !== undefined &&
      (alternativeTextChoice() === "decorative" || usableAlternativeText().length > 0),
  );
  const revoke = (url: string | undefined): void => {
    if (url !== undefined) URL.revokeObjectURL(url);
  };
  const clearLocalPreview = (): void => {
    revoke(localUrl());
    setLocalUrl(undefined);
  };
  const clearSavedPreviews = (): void => {
    revoke(savedBannerUrl);
    savedBannerUrl = undefined;
    setBannerUrl(undefined);
  };

  // Each saved banner ID owns its preview URLs; release old URLs and ignore late prior fetches.
  createEffect(() => {
    const bannerId = savedBannerId();
    clearSavedPreviews();
    if (bannerId === undefined) return;
    let active = true;
    void applicationApi.client
      .fetchCourseBanner(bannerId)
      .then((banner) => {
        if (!active) return;
        savedBannerUrl = URL.createObjectURL(banner);
        setBannerUrl(savedBannerUrl);
      })
      .catch(() => {
        if (active) setMessage("The saved banner previews could not load.");
      });
    onCleanup(() => {
      active = false;
    });
  });
  onCleanup(() => {
    clearLocalPreview();
    clearSavedPreviews();
  });
  function selectFile(event: Event): void {
    const next = (event.currentTarget as HTMLInputElement).files?.item(0) ?? undefined;
    clearLocalPreview();
    setFile(next);
    setSaveState("ready");
    setMessage("");
    if (next !== undefined) setLocalUrl(URL.createObjectURL(next));
  }
  function selectedAlternativeText(): CourseBannerAlternativeText {
    return alternativeTextChoice() === "decorative"
      ? { kind: "decorative" }
      : { kind: "informative", text: usableAlternativeText() };
  }
  async function saveBanner(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const image = file();
    if (image === undefined || !canSave() || saving()) return;
    setSaveState("saving");
    setMessage("");
    try {
      const receipt = await applicationApi.client.uploadCourseBanner(props.courseInstanceId, image);
      const saved = await applicationApi.client.setCourseBanner(props.courseInstanceId, {
        upload: receipt.upload,
        alternativeText: selectedAlternativeText(),
      });
      replaceCourseAppearance(props.courseInstanceId, saved);
      clearLocalPreview();
      setFile(undefined);
      if (fileInput !== undefined) fileInput.value = "";
      setSaveState("saved");
      setMessage("Banner saved.");
    } catch {
      setSaveState("error");
      setMessage("Banner could not save. The saved banner is still displayed.");
    }
  }
  async function removeBanner(): Promise<void> {
    if (savedBanner() === null || saving()) return;
    setSaveState("saving");
    setMessage("");
    try {
      const saved = await applicationApi.client.removeCourseBanner(props.courseInstanceId);
      replaceCourseAppearance(props.courseInstanceId, saved);
      setSaveState("saved");
      setMessage("Banner removed.");
    } catch {
      setSaveState("error");
      setMessage("Banner could not remove. The saved banner is still displayed.");
    }
  }
  return (
    <form class="course-appearance-form" onSubmit={(event) => void saveBanner(event)}>
      <section class="course-appearance-section" aria-labelledby="course-banner-heading">
        <fieldset class="course-appearance-fieldset" disabled={saving()}>
          <legend id="course-banner-heading">Banner</legend>
          <p class="course-appearance-help">
            Upload a 5:1 PNG, JPEG, or WebP image. 1280 by 256 pixels is recommended; larger 5:1
            images are supported. It is not uploaded until you save.
          </p>
          <label class="course-appearance-file-label">
            Banner image
            <input
              data-course-banner-file
              type="file"
              accept="image/png,image/jpeg,image/webp"
              ref={(element) => {
                fileInput = element;
              }}
              onChange={selectFile}
            />
          </label>
          <Show when={localUrl()}>
            {(url) => (
              <div class="course-appearance-banner-preview-group" data-course-banner-local-preview>
                <p>Before saving</p>
                <p>Course entry</p>
                <BannerImage url={url()} alternativeText={selectedAlternativeText()} />
                <p class="course-appearance-banner-course-name">Course: {props.courseLongName}</p>
              </div>
            )}
          </Show>
          <fieldset class="course-appearance-alternative-text">
            <legend>Banner alternative text</legend>
            <label>
              <input
                type="radio"
                name="course-banner-alt"
                checked={alternativeTextChoice() === "decorative"}
                onInput={() => setAlternativeTextChoice("decorative")}
              />{" "}
              Decorative
            </label>
            <label>
              <input
                type="radio"
                name="course-banner-alt"
                checked={alternativeTextChoice() === "informative"}
                onInput={() => setAlternativeTextChoice("informative")}
              />{" "}
              Informative
            </label>
            <Show when={alternativeTextChoice() === "informative"}>
              <label>
                Describe the meaningful course artwork
                <input
                  data-course-banner-alt
                  type="text"
                  maxlength="160"
                  value={alternativeText()}
                  onInput={(event) => setAlternativeText(event.currentTarget.value)}
                  required
                />
              </label>
            </Show>
          </fieldset>
        </fieldset>
        <Show when={savedBanner()}>
          {(banner) => (
            <div class="course-appearance-banner-preview-group" data-course-banner-saved-preview>
              <p>Saved banner</p>
              <p>Course entry</p>
              <Show when={bannerUrl()}>
                {(url) => <BannerImage url={url()} alternativeText={banner().alternativeText} />}
              </Show>
              <p class="course-appearance-banner-course-name">Course: {props.courseLongName}</p>
            </div>
          )}
        </Show>
        <div class="course-appearance-save-actions">
          <button type="submit" disabled={!canSave() || saving()}>
            {saving()
              ? "Saving banner..."
              : savedBanner() === null
                ? "Save banner"
                : "Replace banner"}
          </button>
          <Show when={savedBanner() !== null}>
            <button type="button" disabled={saving()} onClick={() => void removeBanner()}>
              Remove banner
            </button>
          </Show>
          <Show when={message()}>
            <p
              class={
                saveState() === "error" ? "course-appearance-error" : "course-appearance-save-note"
              }
              role={saveState() === "error" ? "alert" : "status"}
            >
              {message()}
            </p>
          </Show>
        </div>
      </section>
    </form>
  );
}

/** Course-scoped Instructor page; theme and banner have separate persistence actions. */
export function CourseAppearancePage(): JSX.Element {
  const route = useRouteScopeData();
  const loadState = useRouteScopeLoadState();
  const retryScope = useRetryRouteScope();
  const course = createMemo(() => {
    const data = route();
    return data?.kind === "course" ? courseRouteView(data) : undefined;
  });
  const storedAppearance = createMemo(() => course()?.appearance);
  return (
    <Show
      when={course()?.summary}
      keyed
      fallback={
        <Show
          when={loadState() === "pending"}
          fallback={
            <Show
              when={loadState() === "rejected"}
              fallback={
                <PageFrame routeSurface="courseAppearance" title="Course Appearance unavailable" />
              }
            >
              <PageFrame routeSurface="courseAppearance" title="Course Appearance could not load">
                <p role="alert">
                  Your saved Course Appearance could not be loaded. Check your connection and try
                  again.
                </p>
                <button type="button" onClick={retryScope}>
                  Retry loading Course Appearance
                </button>
              </PageFrame>
            </Show>
          }
        >
          <PageFrame routeSurface="courseAppearance" title="Loading Course Appearance">
            <p aria-busy="true" aria-live="polite">
              Loading saved Course Appearance...
            </p>
          </PageFrame>
        </Show>
      }
    >
      {(summary) => {
        const appearance = (): CourseAppearanceView => {
          const current = storedAppearance();
          if (current === undefined) throw new Error("Course Appearance is unavailable.");
          return current;
        };
        return (
          <PageFrame
            routeSurface="courseAppearance"
            title="Course Appearance"
            lede="Choose the palette and banner used throughout this Course Instance."
          >
            <style>{COURSE_APPEARANCE_STYLES}</style>
            <style>{COURSE_BANNER_5_TO_1_STYLES}</style>
            <AppearanceThemeEditor courseInstanceId={summary.id} storedAppearance={appearance} />
            <AppearanceBannerEditor
              courseInstanceId={summary.id}
              courseLongName={summary.longName}
              storedAppearance={appearance}
            />
          </PageFrame>
        );
      }}
    </Show>
  );
}
