// screenshot_galleries.ts - Deterministic galleries for the current screenshot folders.

import path from "node:path";
import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";

import type { CaptureManifest, CaptureRecord, ScreenshotRole, ViewportId } from "./manifest";

interface ScreenshotFolder {
  readonly id: string;
  readonly title: string;
  readonly role: ScreenshotRole;
  readonly viewport?: ViewportId;
}

export const SCREENSHOT_GALLERY_FOLDERS: ReadonlyArray<ScreenshotFolder> = [
  { id: "public-laptop", title: "Public laptop screenshots", role: "public", viewport: "laptop" },
  { id: "public-phone", title: "Public phone screenshots", role: "public", viewport: "phone" },
  { id: "instructor", title: "Instructor screenshots", role: "instructor" },
  {
    id: "student-laptop",
    title: "Student laptop screenshots",
    role: "student",
    viewport: "laptop",
  },
  {
    id: "student-tablet",
    title: "Student tablet screenshots",
    role: "student",
    viewport: "tablet",
  },
  { id: "student-phone", title: "Student phone screenshots", role: "student", viewport: "phone" },
  {
    id: "student-square",
    title: "Student square screenshots",
    role: "student",
    viewport: "square",
  },
  { id: "sysadmin", title: "Sysadmin screenshots", role: "sysadmin" },
];

function capturesForFolder(
  manifest: CaptureManifest,
  folder: ScreenshotFolder,
): ReadonlyArray<CaptureRecord> {
  return manifest.captures
    .filter(
      (capture) =>
        capture.role === folder.role &&
        (folder.viewport === undefined || capture.viewport === folder.viewport),
    )
    .sort((left, right) => left.gallery.order - right.gallery.order);
}

function renderFolderPage(manifest: CaptureManifest, folder: ScreenshotFolder): string {
  const lines = [
    `# ${folder.title}`,
    "",
    "Generated from the current screenshot manifest. Images link to their full-size files.",
    "",
    "[Complete screenshot atlas](../SCREENSHOT_ATLAS.md)",
    "",
  ];
  for (const capture of capturesForFolder(manifest, folder)) {
    const image = `../screenshots/${capture.path}`;
    const caption = capture.gallery.caption.replace(/\//gu, " or ");
    const alt = `Screenshot preview of ${caption}`;
    lines.push(
      `[![${alt}](${image})](${image})`,
      "",
      `**${caption}.** ${capture.state} - ${capture.viewport}.`,
      "",
    );
  }
  return `${lines.join("\n")}\n`;
}

export function renderScreenshotGalleries(
  manifest: CaptureManifest,
): Readonly<Record<string, string>> {
  return Object.fromEntries(
    SCREENSHOT_GALLERY_FOLDERS.map((folder) => [
      `${folder.id}.md`,
      renderFolderPage(manifest, folder),
    ]),
  );
}

export async function writeScreenshotGalleries(
  docsRoot: string,
  manifest: CaptureManifest,
): Promise<void> {
  const galleryRoot = path.join(docsRoot, "screenshot_galleries");
  await mkdir(galleryRoot, { recursive: true });
  await Promise.all(
    Object.entries(renderScreenshotGalleries(manifest)).map(([filename, content]) =>
      writeFile(path.join(galleryRoot, filename), content, "utf8"),
    ),
  );
}

export async function verifyScreenshotGalleries(
  galleryRoot: string,
  manifest: CaptureManifest,
): Promise<void> {
  const expected = renderScreenshotGalleries(manifest);
  const entries = await readdir(galleryRoot, { withFileTypes: true });
  const actualNames = entries.map((entry) => entry.name).sort();
  const expectedNames = Object.keys(expected).sort();
  const missing = expectedNames.filter((name) => !actualNames.includes(name));
  const unmanaged = actualNames.filter((name) => !expectedNames.includes(name));
  if (missing.length > 0 || unmanaged.length > 0 || entries.some((entry) => !entry.isFile())) {
    throw new Error(
      `screenshot gallery path set differs; missing=${missing.join(",") || "none"}; ` +
        `unmanaged=${unmanaged.join(",") || "none"}`,
    );
  }
  for (const [filename, content] of Object.entries(expected)) {
    if ((await readFile(path.join(galleryRoot, filename), "utf8")) !== content) {
      throw new Error(`screenshot gallery is not the deterministic folder gallery: ${filename}`);
    }
  }
}
