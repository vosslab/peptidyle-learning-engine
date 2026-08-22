// WP-CA6 behavior tests for instructor draft and same-origin transport.

import assert from "node:assert/strict";
import test from "node:test";

import {
  courseAppearanceDraftChanged,
  courseAppearanceDraftWithAlternativeText,
  courseAppearanceDraftWithCurrentBanner,
  courseAppearanceDraftWithRemoval,
  courseAppearanceDraftWithReplacement,
  courseAppearanceDraftWithTheme,
  courseAppearanceUpdate,
  initialCourseAppearanceDraft,
  validateCourseAppearanceDraft,
} from "../src/features/course_appearance/course_appearance_model.ts";
import {
  ApiProtocolError,
  CourseAppearanceConflictError,
  createHttpApiClient,
} from "../src/api/http_client.ts";

const COURSE_ID = "0198e000-0000-7000-8000-000000000014";
const BANNER_ID = "0198e000-0000-7000-8000-000000000801";
const CANDIDATE_ID = "0198e000-0000-7000-8000-000000000802";

const currentWithoutBanner = {
  theme: "grass",
  revision: "7",
  banner: null,
};

const currentWithBanner = {
  theme: "ocean",
  revision: "11",
  banner: {
    id: BANNER_ID,
    alternativeText: { kind: "informative", text: "Students working beside the lake" },
  },
};

function appearanceResponse(value, status = 200, etag = `"${value.revision}"`) {
  return new Response(JSON.stringify(value), {
    status,
    headers: {
      "cache-control": "no-store",
      "content-type": "application/json; charset=utf-8",
      etag,
    },
  });
}

function bannerResponse(bytes = new Uint8Array([82, 73, 70, 70]), headerChanges = {}) {
  const headers = {
    "cache-control": "no-store",
    "content-disposition": 'attachment; filename="ple-course-banner.webp"',
    "content-length": String(bytes.byteLength),
    "content-type": "image/webp",
    "cross-origin-resource-policy": "same-origin",
    "referrer-policy": "no-referrer",
    "x-content-type-options": "nosniff",
    ...headerChanges,
  };
  return new Response(bytes, { headers });
}

test("theme and banner edits compile into one atomic update without server-private fields", () => {
  const initial = initialCourseAppearanceDraft(currentWithoutBanner);
  assert.equal(courseAppearanceDraftChanged(initial, currentWithoutBanner), false);

  const themed = courseAppearanceDraftWithTheme(initial, "forest");
  assert.equal(courseAppearanceDraftChanged(themed, currentWithoutBanner), true);
  assert.deepEqual(courseAppearanceUpdate(themed), {
    theme: "forest",
    banner: { kind: "remove" },
  });

  const replacement = courseAppearanceDraftWithReplacement(themed, currentWithoutBanner, {
    name: "biology-course.png",
    mediaType: "image/png",
    size: 1_024,
  });
  const informative = courseAppearanceDraftWithAlternativeText(replacement, {
    kind: "informative",
    text: "A labeled chromosome spread",
  });
  assert.deepEqual(validateCourseAppearanceDraft(informative), { valid: true, errors: {} });
  const update = courseAppearanceUpdate(informative, CANDIDATE_ID);
  assert.deepEqual(update, {
    theme: "forest",
    banner: {
      kind: "replace",
      candidate: CANDIDATE_ID,
      alternativeText: { kind: "informative", text: "A labeled chromosome spread" },
    },
  });
  assert.equal(JSON.stringify(update).includes("biology-course.png"), false);
  assert.throws(() => courseAppearanceUpdate(informative), /must upload before saving/u);
});

test("remove is pending until save and can restore the exact current banner", () => {
  const initial = initialCourseAppearanceDraft(currentWithBanner);
  const removed = courseAppearanceDraftWithRemoval(initial, currentWithBanner);
  assert.equal(removed.banner.kind, "remove");
  assert.equal(courseAppearanceDraftChanged(removed, currentWithBanner), true);
  assert.deepEqual(courseAppearanceUpdate(removed), {
    theme: "ocean",
    banner: { kind: "remove" },
  });

  const restored = courseAppearanceDraftWithCurrentBanner(removed, currentWithBanner);
  assert.equal(courseAppearanceDraftChanged(restored, currentWithBanner), false);
  assert.deepEqual(restored, initial);
});

test("local validation keeps unsupported images and incomplete informative text out of fetch", () => {
  const initial = initialCourseAppearanceDraft(currentWithoutBanner);
  const unsupported = courseAppearanceDraftWithReplacement(initial, currentWithoutBanner, {
    name: "active.svg",
    mediaType: "image/svg+xml",
    size: 500,
  });
  assert.equal(validateCourseAppearanceDraft(unsupported).valid, false);
  assert.match(validateCourseAppearanceDraft(unsupported).errors.bannerFile, /JPEG, PNG, or WebP/u);

  const oversized = courseAppearanceDraftWithReplacement(initial, currentWithoutBanner, {
    name: "large.png",
    mediaType: "image/png",
    size: 2 * 1_024 * 1_024 + 1,
  });
  assert.equal(validateCourseAppearanceDraft(oversized).valid, false);
  assert.match(validateCourseAppearanceDraft(oversized).errors.bannerFile, /2 MiB/u);

  const blankAlternative = courseAppearanceDraftWithAlternativeText(
    courseAppearanceDraftWithReplacement(initial, currentWithoutBanner, {
      name: "course.png",
      mediaType: "image/png",
      size: 500,
    }),
    { kind: "informative", text: "   " },
  );
  assert.equal(validateCourseAppearanceDraft(blankAlternative).valid, false);
  assert.match(validateCourseAppearanceDraft(blankAlternative).errors.alternativeText, /1 to 160/u);
});

test("HTTP appearance client uploads opaque bytes then saves with exact CAS and no filename", async () => {
  const requests = [];
  const saved = {
    theme: "forest",
    revision: "8",
    banner: {
      id: BANNER_ID,
      alternativeText: { kind: "decorative" },
    },
  };
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      if (request.method === "GET") return appearanceResponse(currentWithoutBanner);
      if (request.url.endsWith("/banner-candidates")) {
        return new Response(JSON.stringify({ candidate: CANDIDATE_ID }), {
          status: 201,
          headers: {
            "cache-control": "no-store",
            "content-type": "application/json",
          },
        });
      }
      return appearanceResponse(saved);
    },
  });

  assert.deepEqual(await client.getCourseAppearance(COURSE_ID), currentWithoutBanner);
  const image = new Blob([new Uint8Array([1, 2, 3, 4])], { type: "image/png" });
  const receipt = await client.uploadCourseBannerCandidate(COURSE_ID, image);
  assert.equal(receipt.candidate, CANDIDATE_ID);
  const update = {
    theme: "forest",
    banner: {
      kind: "replace",
      candidate: receipt.candidate,
      alternativeText: { kind: "decorative" },
    },
  };
  assert.deepEqual(await client.saveCourseAppearance(COURSE_ID, update, "7"), saved);

  const upload = requests.find((request) => request.url.endsWith("/banner-candidates"));
  const save = requests.find((request) => request.method === "PUT");
  assert.notEqual(upload, undefined);
  assert.notEqual(save, undefined);
  assert.equal(upload.method, "POST");
  assert.equal(upload.credentials, "same-origin");
  assert.equal(upload.cache, "no-store");
  assert.equal(upload.headers.get("content-type"), "image/png");
  assert.deepEqual(new Uint8Array(await upload.arrayBuffer()), new Uint8Array([1, 2, 3, 4]));
  assert.equal(save.headers.get("if-match"), '"7"');
  assert.equal(save.headers.get("content-type"), "application/json");
  const saveBody = await save.text();
  assert.deepEqual(JSON.parse(saveBody), update);
  assert.equal(saveBody.includes("filename"), false);
});

test("HTTP course banner delivery stays same-origin and returns one bounded WebP Blob", async () => {
  const requests = [];
  const bytes = new Uint8Array([82, 73, 70, 70]);
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      requests.push(new Request(new URL(String(input), "https://ple.example"), init));
      return bannerResponse(bytes);
    },
  });

  const blob = await client.fetchCourseBanner(BANNER_ID);
  assert.equal(blob.type, "image/webp");
  assert.deepEqual(new Uint8Array(await blob.arrayBuffer()), bytes);
  assert.equal(requests[0].url, `https://ple.example/api/course-banners/${BANNER_ID}/delivery`);
  assert.equal(requests[0].method, "POST");
  assert.equal(requests[0].credentials, "same-origin");
  assert.equal(requests[0].cache, "no-store");
  assert.equal(requests[0].headers.get("accept"), "image/webp");
  assert.equal(await requests[0].text(), "");
});

test("HTTP course banner delivery rejects cache, type, origin, and length drift", async () => {
  const invalidHeaders = [
    { "cache-control": "private" },
    { "content-type": "image/png" },
    { "cross-origin-resource-policy": "cross-origin" },
    { "x-content-type-options": "" },
    { "content-length": "5" },
  ];
  for (const headerChanges of invalidHeaders) {
    const client = createHttpApiClient({
      fetch: () => Promise.resolve(bannerResponse(undefined, headerChanges)),
    });
    await assert.rejects(client.fetchCourseBanner(BANNER_ID), ApiProtocolError);
  }
});

test("HTTP appearance client distinguishes stale state and rejects weak response metadata", async () => {
  const conflict = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        new Response(JSON.stringify({ error: "changed" }), {
          status: 412,
          headers: { "cache-control": "no-store", "content-type": "application/json" },
        }),
      ),
  });
  await assert.rejects(
    conflict.saveCourseAppearance(COURSE_ID, { theme: "grass", banner: { kind: "remove" } }, "1"),
    CourseAppearanceConflictError,
  );

  const mismatchedEtag = createHttpApiClient({
    fetch: () => Promise.resolve(appearanceResponse(currentWithoutBanner, 200, '"9"')),
  });
  await assert.rejects(mismatchedEtag.getCourseAppearance(COURSE_ID), ApiProtocolError);

  const cacheable = createHttpApiClient({
    fetch: () =>
      Promise.resolve(
        new Response(JSON.stringify(currentWithoutBanner), {
          headers: { "content-type": "application/json", etag: '"7"' },
        }),
      ),
  });
  await assert.rejects(cacheable.getCourseAppearance(COURSE_ID), ApiProtocolError);
});
