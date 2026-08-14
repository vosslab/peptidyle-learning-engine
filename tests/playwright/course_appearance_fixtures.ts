// Shared built-browser fixtures for course-appearance behavior and visual acceptance.

import { deflateSync } from "node:zlib";

import type { Page, Route } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";

export const COURSE_ID = publishedProblemFixture.course.id;
export const COURSE_REFERENCE = `C-${publishedProblemFixture.course.publicId}`;
export const ASSIGNMENT_REFERENCE = `A-${publishedProblemFixture.assignment.publicId}`;
export const APPEARANCE_PATH = `/instructor/courses/${COURSE_REFERENCE}/appearance`;
export const CANDIDATE_ID = "0198e000-0000-7000-8000-000000000811";
export const SECOND_CANDIDATE_ID = "0198e000-0000-7000-8000-000000000812";
export const BANNER_ID = "0198e000-0000-7000-8000-000000000813";

function crc32(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function pngChunk(type: string, data: Uint8Array): Buffer {
  const name = Buffer.from(type, "ascii");
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const checksum = Buffer.alloc(4);
  checksum.writeUInt32BE(crc32(Buffer.concat([name, data])));
  return Buffer.concat([length, name, data, checksum]);
}

function solidPng(width: number, height: number): Buffer {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width, 0);
  header.writeUInt32BE(height, 4);
  header[8] = 8;
  header[9] = 2;
  const scanline = Buffer.alloc(width * 3 + 1);
  for (let index = 1; index < scanline.length; index += 3) {
    scanline[index] = 0x00;
    scanline[index + 1] = 0x88;
    scanline[index + 2] = 0x52;
  }
  const pixels = Buffer.concat(Array.from({ length: height }, () => scanline));
  return Buffer.concat([
    Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]),
    pngChunk("IHDR", header),
    pngChunk("IDAT", deflateSync(pixels)),
    pngChunk("IEND", Buffer.alloc(0)),
  ]);
}

export const bannerBytes = solidPng(1_200, 328);

export function json(
  route: Route,
  value: unknown,
  status = 200,
  headers: Record<string, string> = {},
): Promise<void> {
  return route.fulfill({
    status,
    contentType: "application/json",
    headers,
    body: JSON.stringify(value),
  });
}

export function session(roles: ReadonlyArray<string>): unknown {
  return {
    authenticated: true,
    tenant: publishedProblemFixture.course.tenant,
    user: {
      id: "0198e000-0000-7000-8000-000000000114",
      displayName: "Course instructor",
      roles,
    },
  };
}

export function appearanceHeaders(revision: string): Record<string, string> {
  return { "cache-control": "no-store", etag: `"${revision}"` };
}

/** Resolves the one visible course reference used by appearance browser fixtures. */
export function resolveCourseReference(route: Route, path: string): Promise<void> | null {
  return path === `/api/navigation/${COURSE_REFERENCE}`
    ? json(route, { kind: "course", courseId: COURSE_ID })
    : null;
}

export async function openAppearance(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.goto("/");
  await page.evaluate((path: string) => {
    history.pushState({}, "", path);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, APPEARANCE_PATH);
}
