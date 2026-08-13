/**
 * Opt-in documentation screenshot capture for the real UI walkthrough.
 *
 * The explicit private walkthrough input owns the optional directory. Ordinary
 * Playwright tests pass no directory, so their artifacts remain unchanged.
 */

import { chmod, lstat } from "node:fs/promises";
import path from "node:path";

import type { Locator, Page } from "@playwright/test";

const CAPTURE_DIRECTORY_PARENT = "/private/tmp";
const CAPTURE_DIRECTORY_PREFIX = "ple-docs-screenshots.";
const CANONICAL_VIEWPORT = { width: 1_280, height: 800 } as const;

export const documentationScreenshotNames = [
  "instructor_course_overview.png",
  "instructor_roster_active_student.png",
  "instructor_problem_catalog.png",
  "instructor_assignment_settings.png",
  "instructor_assignment_created.png",
  "genetics_chapter_one_overview.png",
  "student_assignment_list.png",
  "student_timed_problem.png",
  "student_fresh_practice.png",
  "student_retake_fresh_problem.png",
  "instructor_gradebook_mastery_loop.png",
] as const;

export type DocumentationScreenshotName = (typeof documentationScreenshotNames)[number];

export function documentationScreenshotsEnabled(screenshotDirectory?: string): boolean {
  return screenshotDirectory !== undefined;
}

async function validateCaptureDirectory(directory: string): Promise<void> {
  if (!path.isAbsolute(directory)) {
    throw new Error("documentation screenshot directory must be absolute");
  }
  if (
    path.dirname(directory) !== CAPTURE_DIRECTORY_PARENT ||
    !path.basename(directory).startsWith(CAPTURE_DIRECTORY_PREFIX)
  ) {
    throw new Error("documentation screenshot directory must be runner-created under /private/tmp");
  }
  const metadata = await lstat(directory);
  if (!metadata.isDirectory() || metadata.isSymbolicLink()) {
    throw new Error("documentation screenshot directory must be a regular directory");
  }
  const getuid = process.getuid;
  if (getuid === undefined || metadata.uid !== getuid()) {
    throw new Error("documentation screenshot directory must be owned by this user");
  }
  if ((metadata.mode & 0o777) !== 0o700) {
    throw new Error("documentation screenshot directory must have mode 0700");
  }
}

async function validateScreenshotPath(filePath: string): Promise<void> {
  try {
    const metadata = await lstat(filePath);
    if (metadata.isSymbolicLink() || !metadata.isFile()) {
      throw new Error("documentation screenshot path must be a regular file");
    }
    throw new Error("documentation screenshot path already exists");
  } catch (error: unknown) {
    if (isMissingPathError(error)) return;
    throw error;
  }
}

function isMissingPathError(error: unknown): boolean {
  return typeof error === "object" && error !== null && "code" in error && error.code === "ENOENT";
}

/** Capture one bounded public UI state only when the dedicated launcher opts in. */
export async function captureDocumentationScreenshot(
  page: Page,
  screenshotName: DocumentationScreenshotName,
  anchor?: Locator,
  cropTopPixels?: number,
  screenshotDirectory?: string,
  anchorAlignment: "top" | "bottom" = "top",
): Promise<void> {
  if (screenshotDirectory === undefined) return;
  if (!documentationScreenshotNames.includes(screenshotName)) {
    throw new Error("documentation screenshot name is not approved");
  }
  if (
    cropTopPixels !== undefined &&
    (!Number.isInteger(cropTopPixels) ||
      cropTopPixels < 0 ||
      cropTopPixels >= CANONICAL_VIEWPORT.height)
  ) {
    throw new Error(
      "documentation screenshot crop must be a whole number below the viewport height",
    );
  }
  await validateCaptureDirectory(screenshotDirectory);
  const filePath = path.join(screenshotDirectory, screenshotName);
  await validateScreenshotPath(filePath);
  const viewportHeight = CANONICAL_VIEWPORT.height - (cropTopPixels ?? 0);
  const anchorMargin = cropTopPixels === undefined ? 72 : 0;
  await page.setViewportSize({ width: CANONICAL_VIEWPORT.width, height: viewportHeight });
  if (anchor !== undefined) {
    await anchor.scrollIntoViewIfNeeded();
    await anchor.evaluate(
      (element, options) => {
        const bounds = element.getBoundingClientRect();
        const target =
          options.alignment === "bottom"
            ? bounds.bottom - window.innerHeight + options.margin
            : bounds.top - options.margin;
        window.scrollBy(0, target);
      },
      { alignment: anchorAlignment, margin: anchorMargin },
    );
  }
  const devicePixelRatio = await page.evaluate(() => window.devicePixelRatio);
  if (devicePixelRatio !== 1) {
    throw new Error("documentation screenshots require device pixel ratio 1");
  }
  await page.screenshot({
    path: filePath,
    animations: "disabled",
    caret: "hide",
    scale: "css",
  });
  const metadata = await lstat(filePath);
  if (metadata.isSymbolicLink() || !metadata.isFile() || metadata.size === 0) {
    throw new Error("documentation screenshot was not written as a nonempty regular file");
  }
  await chmod(filePath, 0o644);
}
