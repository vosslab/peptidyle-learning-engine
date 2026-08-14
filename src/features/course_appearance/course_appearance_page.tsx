// course_appearance_page.tsx - instructor theme and entry-banner settings.

import { A, revalidate } from "@solidjs/router";
import {
  For,
  Show,
  createEffect,
  createMemo,
  createResource,
  createSignal,
  onCleanup,
  type JSX,
} from "solid-js";

import type { CourseAppearance } from "../../../generated/api/CourseAppearance";
import type { CourseBannerAlternativeText } from "../../../generated/api/CourseBannerAlternativeText";
import type { CourseBannerCandidateId } from "../../../generated/api/CourseBannerCandidateId";
import type { CourseThemeId } from "../../../generated/api/CourseThemeId";
import { useApiRuntime } from "../../api/runtime";
import { CourseManagementNav } from "../../components/course_management_nav";
import {
  ApiRequestError,
  CourseAppearanceConflictError,
  CourseAppearanceFileError,
} from "../../api/http_client";
import {
  courseRouteData,
  useCourseThemePresentation,
  useCourseThemeRouteData,
} from "./course_theme_context";
import {
  courseAppearanceBannerWillDisplay,
  courseAppearanceDraftAlternativeText,
  courseAppearanceDraftChanged,
  courseAppearanceDraftWithAlternativeText,
  courseAppearanceDraftWithCurrentBanner,
  courseAppearanceDraftWithRemoval,
  courseAppearanceDraftWithReplacement,
  courseAppearanceDraftWithTheme,
  courseAppearanceUpdate,
  initialCourseAppearanceDraft,
  validateCourseAppearanceDraft,
  type CourseAppearanceDraft,
  type CourseAppearanceDraftErrors,
} from "./course_appearance_model";
import { createCourseAppearanceRepository } from "./course_appearance_repository";
import { COURSE_APPEARANCE_STYLES } from "./course_appearance_styles";
import { COURSE_THEME_OPTIONS, courseThemeStyle, courseThemeTokens } from "./theme_catalog";
import { courseRouteReference } from "../../navigation/public_route";

type SavePhase = "ready" | "uploading" | "saving" | "reloading";

interface UploadedCandidate {
  readonly file: File;
  readonly candidate: CourseBannerCandidateId;
}

interface BannerPreviewProps {
  readonly courseTitle: string;
  readonly label: string;
  readonly width: "wide" | "narrow";
  readonly source: string | null;
  readonly alternativeText: CourseBannerAlternativeText | null;
}

function alternativeTextValue(alternativeText: CourseBannerAlternativeText | null): string {
  return alternativeText?.kind === "informative" ? alternativeText.text : "";
}

function BannerPreview(props: BannerPreviewProps): JSX.Element {
  return (
    <figure class={`course-appearance-preview course-appearance-preview--${props.width}`}>
      <figcaption>{props.label}</figcaption>
      <p class="course-appearance-preview-title">{props.courseTitle}</p>
      <Show
        when={props.source}
        keyed
        fallback={
          <p class="course-appearance-no-banner">No banner will appear on the course entry page.</p>
        }
      >
        {(source) => (
          <img
            class="course-appearance-banner"
            src={source}
            alt={alternativeTextValue(props.alternativeText)}
            width="1200"
            height="328"
          />
        )}
      </Show>
    </figure>
  );
}

function safeErrorMessage(error: unknown): string {
  if (error instanceof CourseAppearanceFileError) return error.message;
  if (error instanceof ApiRequestError) {
    switch (error.status) {
      case 401:
        return "Your session ended. Sign in again, then save the preserved settings.";
      case 403:
        return "You no longer have permission to change this course. Your local choices remain visible.";
      case 404:
        return "This course is no longer available to this account. Your local choices remain visible.";
      case 413:
        return "The selected image is too large. Choose a smaller JPEG, PNG, or WebP image.";
      case 415:
        return "The selected file is not a supported JPEG, PNG, or WebP image.";
      case 422:
        return "The server could not use that image or setting. Review the fields and try again.";
      default:
        return "The course appearance could not be saved. Your choices are still here; try again.";
    }
  }
  return "The course appearance could not be saved. Your choices are still here; try again.";
}

/** Working course-local settings form; no answer-bearing value or object key enters this surface. */
export function CourseAppearancePage(): JSX.Element {
  const runtime = useApiRuntime();
  const routeThemeData = useCourseThemeRouteData();
  const updateRoutePresentation = useCourseThemePresentation();
  if (routeThemeData === undefined) {
    return (
      <section class="page" data-route-surface="courseAppearance">
        <p class="eyebrow">Course settings</p>
        <h1>Course appearance is not available for this account</h1>
        <p class="page-lede">Only a course instructor can change its appearance.</p>
        <A class="quiet-link" href="/">
          Return to courses
        </A>
      </section>
    );
  }
  const course = courseRouteData(routeThemeData);
  if (course.summary.role !== "instructor") {
    return (
      <section class="page" data-route-surface="courseAppearance">
        <p class="eyebrow">Course settings</p>
        <h1>Course appearance is not available for this account</h1>
        <p class="page-lede">Only a course instructor can change its appearance.</p>
        <A class="quiet-link" href={`/courses/${courseRouteReference(course.summary.publicId)}`}>
          Return to course
        </A>
      </section>
    );
  }

  const repository = createCourseAppearanceRepository(runtime.client);
  const [current, setCurrent] = createSignal<CourseAppearance>(course.appearance);
  const [draft, setDraft] = createSignal<CourseAppearanceDraft>(
    initialCourseAppearanceDraft(course.appearance),
  );
  const [phase, setPhase] = createSignal<SavePhase>("ready");
  const [replacementFile, setReplacementFile] = createSignal<File | null>(null);
  const [replacementPreview, setReplacementPreview] = createSignal<string | null>(null);
  const [uploadedCandidate, setUploadedCandidate] = createSignal<UploadedCandidate | null>(null);
  const [fieldErrors, setFieldErrors] = createSignal<CourseAppearanceDraftErrors>({});
  const [errorMessage, setErrorMessage] = createSignal<string | null>(null);
  const [successMessage, setSuccessMessage] = createSignal<string | null>(null);
  const [conflict, setConflict] = createSignal(false);
  const [savedBannerUrl] = createResource(
    () => current().banner?.id ?? null,
    (assetId) => runtime.client.issueProtectedAssetDelivery(assetId),
  );
  let fileInput: HTMLInputElement | undefined;
  let errorHeading: HTMLHeadingElement | undefined;

  const busy = (): boolean => phase() !== "ready";
  const changed = (): boolean => courseAppearanceDraftChanged(draft(), current());
  const displayedAlternativeText = createMemo(() => courseAppearanceDraftAlternativeText(draft()));
  const replacementFileName = createMemo(() => {
    const banner = draft().banner;
    return banner.kind === "replace" ? banner.file.name : null;
  });
  const previewSource = createMemo(() => {
    if (!courseAppearanceBannerWillDisplay(draft())) return null;
    const local = replacementPreview();
    if (local !== null) return local;
    const banner = draft().banner;
    return banner.kind === "keep" ? (savedBannerUrl() ?? null) : null;
  });

  createEffect(() => {
    const file = replacementFile();
    if (file === null) {
      setReplacementPreview(null);
      return;
    }
    const source = URL.createObjectURL(file);
    setReplacementPreview(source);
    onCleanup(() => URL.revokeObjectURL(source));
  });

  function clearMessages(): void {
    setErrorMessage(null);
    setSuccessMessage(null);
    setConflict(false);
  }

  function clearSelectedFile(): void {
    setReplacementFile(null);
    setUploadedCandidate(null);
    if (fileInput !== undefined) fileInput.value = "";
  }

  function editTheme(theme: CourseThemeId): void {
    if (busy()) return;
    clearMessages();
    setDraft((value) => courseAppearanceDraftWithTheme(value, theme));
  }

  function chooseFile(event: Event & { currentTarget: HTMLInputElement }): void {
    if (busy()) return;
    const file = event.currentTarget.files?.item(0) ?? null;
    if (file === null) return;
    clearMessages();
    setReplacementFile(file);
    setUploadedCandidate(null);
    const next = courseAppearanceDraftWithReplacement(draft(), current(), {
      name: file.name,
      mediaType: file.type,
      size: file.size,
    });
    setDraft(next);
    setFieldErrors(validateCourseAppearanceDraft(next).errors);
  }

  function cancelBannerChange(): void {
    if (busy()) return;
    clearMessages();
    clearSelectedFile();
    setFieldErrors({});
    setDraft((value) => courseAppearanceDraftWithCurrentBanner(value, current()));
  }

  function removeBanner(): void {
    if (busy()) return;
    clearMessages();
    clearSelectedFile();
    setFieldErrors({});
    setDraft((value) => courseAppearanceDraftWithRemoval(value, current()));
  }

  function setAlternativeText(alternativeText: CourseBannerAlternativeText): void {
    if (busy()) return;
    clearMessages();
    setDraft((value) => courseAppearanceDraftWithAlternativeText(value, alternativeText));
  }

  function focusFirstError(errors: CourseAppearanceDraftErrors): void {
    queueMicrotask(() => {
      if (errors.bannerFile !== undefined) fileInput?.focus();
      else if (errors.alternativeText !== undefined) {
        document.getElementById("course-banner-alt-text")?.focus();
      } else errorHeading?.focus();
    });
  }

  async function candidateForSave(): Promise<CourseBannerCandidateId | undefined> {
    if (draft().banner.kind !== "replace") return undefined;
    const file = replacementFile();
    if (file === null) throw new CourseAppearanceFileError("Choose the replacement image again");
    const prior = uploadedCandidate();
    if (prior?.file === file) return prior.candidate;
    setPhase("uploading");
    setSuccessMessage("Preparing the selected banner...");
    const receipt = await repository.uploadBanner(course.summary.id, file);
    setUploadedCandidate({ file, candidate: receipt.candidate });
    return receipt.candidate;
  }

  async function refreshThemeScope(): Promise<void> {
    await revalidate(runtime.queries.courseScope.keyFor(course.summary.id));
  }

  async function save(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy() || !changed()) return;
    clearMessages();
    const validation = validateCourseAppearanceDraft(draft());
    setFieldErrors(validation.errors);
    if (!validation.valid) {
      setErrorMessage("Review the highlighted banner details, then save again.");
      focusFirstError(validation.errors);
      return;
    }
    try {
      const candidate = await candidateForSave();
      setPhase("saving");
      setSuccessMessage("Saving course appearance...");
      const saved = await repository.save(
        course.summary.id,
        courseAppearanceUpdate(draft(), candidate),
        current().revision,
      );
      setCurrent(saved);
      updateRoutePresentation?.(saved);
      setDraft(initialCourseAppearanceDraft(saved));
      clearSelectedFile();
      setFieldErrors({});
      setConflict(false);
      setErrorMessage(null);
      setSuccessMessage("Course appearance saved.");
      try {
        await refreshThemeScope();
      } catch {
        setErrorMessage(
          "The settings saved, but the page theme could not refresh. Reload this page to see it.",
        );
      }
    } catch (error: unknown) {
      setSuccessMessage(null);
      if (error instanceof CourseAppearanceConflictError) {
        setConflict(true);
        queueMicrotask(() => errorHeading?.focus());
      } else {
        if (error instanceof ApiRequestError && error.status === 422) {
          setUploadedCandidate(null);
        }
        setErrorMessage(safeErrorMessage(error));
        queueMicrotask(() => errorHeading?.focus());
      }
    } finally {
      setPhase("ready");
    }
  }

  async function reloadCurrent(): Promise<void> {
    if (busy()) return;
    setPhase("reloading");
    setErrorMessage(null);
    setSuccessMessage("Loading the current course appearance...");
    try {
      const newest = await repository.load(course.summary.id);
      setCurrent(newest);
      updateRoutePresentation?.(newest);
      setDraft(initialCourseAppearanceDraft(newest));
      clearSelectedFile();
      setFieldErrors({});
      setConflict(false);
      setSuccessMessage("Current course appearance loaded. Review it before making new changes.");
      try {
        await refreshThemeScope();
      } catch {
        setErrorMessage("The current settings loaded, but the page theme could not refresh.");
      }
    } catch (error: unknown) {
      setSuccessMessage(null);
      setErrorMessage(safeErrorMessage(error));
    } finally {
      setPhase("ready");
    }
  }

  return (
    <section class="page" data-route-surface="courseAppearance">
      <style>{COURSE_APPEARANCE_STYLES}</style>
      <p class="eyebrow">Instructor course design</p>
      <h1>Course appearance</h1>
      <p class="page-lede">
        Give {course.summary.title} a recognizable color theme and optional entry banner. The course
        title stays readable text, and the selected theme applies only inside this course.
      </p>
      <CourseManagementNav coursePublicId={course.summary.publicId} active="appearance" />

      <Show when={conflict()}>
        <section class="course-appearance-conflict" role="alert">
          <h2
            tabindex="-1"
            ref={(element: HTMLHeadingElement) => {
              errorHeading = element;
            }}
          >
            A newer course appearance exists
          </h2>
          <p>Your theme, image, and alternative-text choices are still shown below.</p>
          <button class="quiet-action" type="button" onClick={() => void reloadCurrent()}>
            Review current appearance
          </button>
        </section>
      </Show>
      <Show when={errorMessage()}>
        {(message) => (
          <section class="course-appearance-error" role="alert">
            <h2
              tabindex="-1"
              ref={(element: HTMLHeadingElement) => {
                errorHeading = element;
              }}
            >
              Appearance needs attention
            </h2>
            <p>{message()}</p>
          </section>
        )}
      </Show>
      <Show when={successMessage()}>
        {(message) => (
          <p class="course-appearance-success" role="status" aria-live="polite">
            {message()}
          </p>
        )}
      </Show>

      <form
        class="course-appearance-form"
        aria-busy={busy()}
        onSubmit={(event) => void save(event)}
      >
        <section class="course-appearance-section" aria-labelledby="course-theme-heading">
          <fieldset class="course-appearance-fieldset" disabled={busy()}>
            <legend id="course-theme-heading">Course color theme</legend>
            <p class="course-appearance-help">
              Choose by name. The swatches are a preview, not the only indication of the selection.
            </p>
            <div class="course-appearance-theme-grid">
              <For each={COURSE_THEME_OPTIONS}>
                {(option) => (
                  <label
                    class="course-appearance-theme-card"
                    style={courseThemeStyle(option.tokens)}
                  >
                    <input
                      type="radio"
                      name="course-theme"
                      value={option.id}
                      checked={draft().theme === option.id}
                      onInput={() => editTheme(option.id)}
                    />
                    <span class="course-appearance-theme-label">
                      <span>{option.tokens.name}</span>
                      <span class="course-appearance-swatches" aria-hidden="true">
                        <span
                          class="course-appearance-swatch"
                          style={{ "background-color": option.tokens.anchors.canvas }}
                        />
                        <span
                          class="course-appearance-swatch"
                          style={{ "background-color": option.tokens.anchors.secondary }}
                        />
                        <span
                          class="course-appearance-swatch"
                          style={{ "background-color": option.tokens.anchors.accent }}
                        />
                      </span>
                    </span>
                  </label>
                )}
              </For>
            </div>
          </fieldset>
        </section>

        <section class="course-appearance-section" aria-labelledby="course-banner-heading">
          <h2 id="course-banner-heading">Course entry banner</h2>
          <p class="course-appearance-help">
            Choose a JPEG, PNG, or WebP image up to 2 MiB. The server creates one centered 1200 by
            328 image; both previews scale that exact result without stretching it.
          </p>
          <label class="course-appearance-file">
            {current().banner === null ? "Choose a banner image" : "Choose a replacement banner"}
            <input
              ref={(element: HTMLInputElement) => {
                fileInput = element;
              }}
              type="file"
              accept="image/jpeg,image/png,image/webp"
              disabled={busy()}
              aria-describedby={
                fieldErrors().bannerFile === undefined ? undefined : "course-banner-file-error"
              }
              onChange={chooseFile}
            />
          </label>
          <Show when={replacementFileName()} keyed>
            {(name) => <p class="course-appearance-file-summary">Selected: {name}</p>}
          </Show>
          <Show when={fieldErrors().bannerFile}>
            {(message) => (
              <p id="course-banner-file-error" class="course-appearance-field-error" role="alert">
                {message()}
              </p>
            )}
          </Show>
          <div class="course-appearance-file-actions">
            <Show when={draft().banner.kind === "replace"}>
              <button
                class="quiet-action"
                type="button"
                disabled={busy()}
                onClick={cancelBannerChange}
              >
                Cancel selected banner
              </button>
            </Show>
            <Show when={current().banner !== null && draft().banner.kind !== "remove"}>
              <button class="quiet-action" type="button" disabled={busy()} onClick={removeBanner}>
                Remove banner on save
              </button>
            </Show>
            <Show when={draft().banner.kind === "remove"}>
              <p>Banner removal is ready but not saved.</p>
              <button
                class="quiet-action"
                type="button"
                disabled={busy()}
                onClick={cancelBannerChange}
              >
                Keep current banner instead
              </button>
            </Show>
          </div>

          <Show when={displayedAlternativeText()} keyed>
            {(alternativeText) => (
              <fieldset class="course-appearance-fieldset course-appearance-alt-options">
                <legend>What does the banner communicate?</legend>
                <label>
                  <input
                    type="radio"
                    name="course-banner-alt-kind"
                    checked={alternativeText.kind === "decorative"}
                    disabled={busy()}
                    onInput={() => setAlternativeText({ kind: "decorative" })}
                  />
                  Decorative only; the adjacent course title carries the meaning
                </label>
                <label>
                  <input
                    type="radio"
                    name="course-banner-alt-kind"
                    checked={alternativeText.kind === "informative"}
                    disabled={busy()}
                    onInput={() =>
                      setAlternativeText({
                        kind: "informative",
                        text: alternativeText.kind === "informative" ? alternativeText.text : "",
                      })
                    }
                  />
                  Informative; describe information that is not in the course title
                </label>
                <Show when={displayedAlternativeText()?.kind === "informative"}>
                  <label class="course-appearance-alt-text">
                    Banner alternative text
                    <input
                      id="course-banner-alt-text"
                      value={alternativeTextValue(displayedAlternativeText())}
                      maxlength={160}
                      disabled={busy()}
                      aria-describedby={
                        fieldErrors().alternativeText === undefined
                          ? "course-banner-alt-help"
                          : "course-banner-alt-help course-banner-alt-error"
                      }
                      onInput={(event) =>
                        setAlternativeText({
                          kind: "informative",
                          text: event.currentTarget.value,
                        })
                      }
                    />
                    <span id="course-banner-alt-help" class="course-appearance-help">
                      Describe only visual information that students need and the title does not
                      say.
                    </span>
                  </label>
                </Show>
                <Show when={fieldErrors().alternativeText}>
                  {(message) => (
                    <p
                      id="course-banner-alt-error"
                      class="course-appearance-field-error"
                      role="alert"
                    >
                      {message()}
                    </p>
                  )}
                </Show>
              </fieldset>
            )}
          </Show>
        </section>

        <section class="course-appearance-section" aria-labelledby="appearance-preview-heading">
          <h2 id="appearance-preview-heading">Preview before saving</h2>
          <p class="course-appearance-help">
            Wide and narrow use the same image and aspect ratio. The course title remains text
            outside the banner.
          </p>
          <div
            class="course-appearance-preview-theme"
            data-preview-theme={draft().theme}
            style={courseThemeStyle(courseThemeTokens(draft().theme))}
          >
            <BannerPreview
              courseTitle={course.summary.title}
              label="Wide course entry preview"
              width="wide"
              source={previewSource()}
              alternativeText={displayedAlternativeText()}
            />
            <BannerPreview
              courseTitle={course.summary.title}
              label="Narrow course entry preview"
              width="narrow"
              source={previewSource()}
              alternativeText={displayedAlternativeText()}
            />
          </div>
        </section>

        <div class="course-appearance-save-actions">
          <button class="primary-action" type="submit" disabled={busy() || !changed()}>
            {phase() === "uploading"
              ? "Preparing banner..."
              : phase() === "saving"
                ? "Saving appearance..."
                : phase() === "reloading"
                  ? "Loading current appearance..."
                  : "Save appearance"}
          </button>
          <p class="course-appearance-save-note">
            {changed()
              ? "Changes stay local until you save them."
              : "The displayed appearance matches the saved course."}
          </p>
        </div>
      </form>
    </section>
  );
}
