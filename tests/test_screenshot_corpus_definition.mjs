import assert from "node:assert/strict";
import test from "node:test";

import {
  computeCoverage,
  generateManifest,
} from "../tests/playwright/screenshot_corpus/manifest.ts";
import { SCREENSHOT_SCENARIOS } from "../tests/playwright/screenshot_corpus/scenario_registry.ts";
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
      paths.push(`${scenario.role}/${capture.checkpoint}.png`);
    }
  }
  assert.equal(new Set(ids).size, ids.length);
  assert.equal(new Set(paths).size, paths.length);
  assert.ok(ids.length > 0);
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
