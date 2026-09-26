import assert from "node:assert/strict";
import test from "node:test";

import {
  computeCoverage,
  generateManifest,
  VIEWPORT_IDS,
} from "../tests/playwright/screenshot_corpus/manifest.ts";
import { SCREENSHOT_SCENARIOS } from "../tests/playwright/screenshot_corpus/scenario_registry.ts";
import {
  captureIdentity,
  requiresFreshViewportContext,
} from "../tests/playwright/screenshot_corpus/runtime.ts";
import { COURSE_THEME_VALUES } from "../generated/api/CourseTheme.ts";
import {
  catalogScreenshotFilename,
  TIER_TWO_FILENAME_ALIASES,
} from "../tests/playwright/screenshot_corpus/filenames.ts";
import { ROUTE_CONTRACT } from "../src/route_contract.ts";
import { RIBBON_TASK_CATALOG, TAB_CATALOG } from "../src/ribbon/ribbon_catalog.ts";

function sampleCapture(overrides = {}) {
  return {
    id: "instructor_course_list",
    path: "instructor/course_list.png",
    role: "instructor",
    routeId: "instructorHome",
    area: "courses",
    workflow: "seeded teaching course",
    state: "course list",
    scenario: "instructor_seeded",
    checkpoint: "course_list",
    viewport: "laptop",
    privacyProfile: "instructor_answer_free",
    gallery: { order: 1, caption: "Instructor Course Instances", featured: false },
    ...overrides,
  };
}

test("each screenshot has one role, checkpoint, id, and path", () => {
  const ids = [];
  const paths = [];
  for (const scenario of SCREENSHOT_SCENARIOS) {
    assert.ok(scenario.role);
    const checkpoints = new Set();
    for (const capture of scenario.captures) {
      assert.equal(checkpoints.has(capture.checkpoint), false);
      checkpoints.add(capture.checkpoint);
      ids.push(`${scenario.role}_${capture.checkpoint}`);
      paths.push(
        captureIdentity(scenario.role, capture.checkpoint, capture.viewport, capture.filenameStem)
          .path,
      );
    }
  }
  assert.equal(new Set(ids).size, ids.length);
  assert.equal(new Set(paths).size, paths.length);
  assert.ok(ids.length > 0);
});

test("Student checkpoints have direct captures in every viewport", () => {
  const studentCaptures = SCREENSHOT_SCENARIOS.filter(
    (scenario) => scenario.role === "student",
  ).flatMap((scenario) => scenario.captures);
  const checkpointByViewport = new Map(VIEWPORT_IDS.map((viewport) => [viewport, new Set()]));
  for (const capture of studentCaptures) {
    const checkpoint = capture.checkpoint.replace(/_(laptop|tablet|phone|square)$/u, "");
    checkpointByViewport.get(capture.viewport)?.add(checkpoint);
  }
  const expected = new Set([...checkpointByViewport.values()].flatMap((set) => [...set]));
  const missingByViewport = Object.fromEntries(
    VIEWPORT_IDS.map((viewport) => [
      viewport,
      [...expected].filter((checkpoint) => !checkpointByViewport.get(viewport)?.has(checkpoint)),
    ]),
  );
  const missing = Object.entries(missingByViewport).filter(
    ([, checkpoints]) => checkpoints.length > 0,
  );
  assert.deepEqual(
    missing,
    [],
    `Student viewport parity failed: ${missing
      .map(([viewport, checkpoints]) => `${viewport} missing ${checkpoints.join(", ")}`)
      .join("; ")}`,
  );
});

test("Course theme comparisons use purpose-only filenames for every theme", () => {
  const scenario = SCREENSHOT_SCENARIOS.find(
    (candidate) => candidate.id === "instructor_theme_samples",
  );
  assert.ok(scenario);
  const expected = COURSE_THEME_VALUES.map((theme) => `theme_sample-${theme}`);
  assert.deepEqual(
    scenario.captures.map((capture) => capture.filenameStem).sort(),
    [...expected].sort(),
  );
  assert.ok(scenario.captures.every((capture) => capture.viewport === "laptop"));
  assert.ok(scenario.captures.every((capture) => capture.state.startsWith("theme sample ")));
});

test("tiered screenshot filenames use explicit aliases for every catalog task", () => {
  assert.deepEqual(
    Object.keys(TIER_TWO_FILENAME_ALIASES).sort(),
    RIBBON_TASK_CATALOG.map((item) => item.id).sort(),
  );
  assert.equal(
    catalogScreenshotFilename("courses", "myBlueprintCourses", "blueprint_detail"),
    "courses-blueprint-blueprint_detail",
  );
  assert.equal(
    catalogScreenshotFilename("questions", "searchQuestionLibrary", "filtered_results"),
    "questions-search-filtered_results",
  );
  assert.equal(
    catalogScreenshotFilename("coursework", "activeAttempt", "q_answered_fib"),
    "coursework-active-q_answered_fib",
  );

  const tabSlugs = TAB_CATALOG.map((item) =>
    item.id.replace(/[A-Z]/gu, (c) => `-${c.toLowerCase()}`),
  );
  const taskAliases = Object.values(TIER_TWO_FILENAME_ALIASES);
  const prefixes = tabSlugs.flatMap((tab) => taskAliases.map((task) => `${tab}-${task}-`));
  for (const scenario of SCREENSHOT_SCENARIOS) {
    for (const capture of scenario.captures) {
      if (capture.filenameStem === undefined) continue;
      if (scenario.id === "instructor_theme_samples") {
        assert.match(capture.filenameStem, /^theme_sample-[a-z0-9-]+$/u);
        continue;
      }
      assert.ok(
        prefixes.some((prefix) => capture.filenameStem.startsWith(prefix)),
        `${scenario.id}:${capture.checkpoint} has an unknown Tier 1/Tier 2 filename prefix`,
      );
    }
  }
});

test("a responsive capture cannot resize between desktop and mobile emulation", () => {
  assert.equal(requiresFreshViewportContext("laptop", "phone"), true);
  assert.equal(requiresFreshViewportContext("square", "tablet"), true);
  assert.equal(requiresFreshViewportContext("laptop", "square"), false);
  assert.equal(requiresFreshViewportContext("tablet", "phone"), false);
});

test("a student capture cannot use an instructor-only route", () => {
  assert.throws(
    () =>
      generateManifest(
        [
          sampleCapture({
            id: "student_course_list",
            path: "student/course_list.png",
            role: "student",
            routeId: "instructorHome",
            scenario: "student_course_list",
            checkpoint: "course_list",
          }),
        ],
        { routes: [], ribbonDestinations: [] },
      ),
    /Product Role outside its route contract/,
  );
});

test("an uncovered route without a listed exception is refused", () => {
  assert.throws(
    () => computeCoverage([], { routes: [], ribbonDestinations: [] }),
    /uncovered route /,
  );
});

test("a listed exception covers a route that has no capture", () => {
  const coverage = computeCoverage([], {
    routes: ROUTE_CONTRACT.map((route) => ({
      id: route.id,
      status: "deferred",
      reason: "listed for the corpus definition test",
    })),
    ribbonDestinations: [
      ...TAB_CATALOG.map((control) => ({
        id: `tab:${control.id}`,
        status: "deferred",
        reason: "listed for the corpus definition test",
      })),
      ...RIBBON_TASK_CATALOG.map((control) => ({
        id: `task:${control.id}`,
        status: "deferred",
        reason: "listed for the corpus definition test",
      })),
    ],
  });
  assert.equal(
    coverage.routes.every((entry) => entry.status === "deferred"),
    true,
  );
});
