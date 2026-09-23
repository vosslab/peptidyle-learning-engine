// Student current-Course Ribbon browser contract.
// Selector contract: src/ribbon/app_ribbon.tsx exposes tier-one IDs; the
// current-source App resolves courseScope/assessmentAttemptScope before
// RouteScopeProvider supplies the Course used by src/ribbon/ribbon_contract.ts.

import assert from "node:assert/strict";
import { once } from "node:events";
import { readFileSync } from "node:fs";
import { createServer } from "node:http";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { chromium } from "playwright";

import { CANONICAL_VIEWPORTS } from "./screenshot_corpus/manifest.ts";

const COURSE_INSTANCE_ID = "CI7K3M2QAZ";
const ATTEMPT_COURSE_INSTANCE_ID = "CI4W8QF9AD";
const ASSESSMENT_ATTEMPT_ID = "00000000-0000-0000-0000-000000000001";
const RIBBON_CONTROL_SELECTOR = '[data-ribbon-control-id="%s"]';

const fixtureSource = String.raw`
  import { createComponent } from "solid-js";
  import { render } from "solid-js/web";
  import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
  import { ApplicationApiProvider } from "./src/api/application_api";
  import { App } from "./src/app";
  import { SessionProvider } from "./src/auth/session_context";

  const classification = {
    disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
    subjectUuid: null,
    topicUuid: null,
    subtopicUuid: null,
    tags: [],
  };
  const course = (id) => ({
    summary: {
      id,
      shortName: id === "CI4W8QF9AD" ? "BIOL 302" : "BCHM 301",
      longName: "Authorized current course",
      classification,
      term: { startDate: "2026-08-31", endDate: "2026-12-12" },
      role: "student",
    },
    appearance: { theme: "ocean", banner: null },
  });

  export function mountStudentCoursePinningHarness(target) {
    const history = createMemoryHistory();
    let releaseCourse;
    let releaseAttempt;
    const applicationApi = {
      client: {
        getProfileAvatar: () => Promise.resolve({ avatar: null }),
        listLiveStudentCourses: () => Promise.resolve([
          { id: "CI7K3M2QAZ", shortName: "BCHM 301", longName: "Authorized current course" },
          { id: "CI4W8QF9AD", shortName: "BIOL 302", longName: "Authorized Attempt course" },
        ]),
        listLiveStudentAssessments: () => Promise.resolve([]),
      },
      queries: {
        courseScope: () => new Promise((resolve) => { releaseCourse = resolve; }),
        assessmentAttemptScope: () => new Promise((resolve) => { releaseAttempt = resolve; }),
        assessmentAttemptHistory: () => Promise.reject(new Error("Summary is outside this fixture.")),
      },
    };
    const dispose = render(
      () => createComponent(ApplicationApiProvider, {
        applicationApi,
        get children() {
          return createComponent(SessionProvider, {
            getSession: () => Promise.resolve({
              authenticated: true,
              account: { id: "student-pinning", productRole: "student" },
            }),
            logout: () => Promise.resolve(),
            advanceSessionBoundary: () => undefined,
            get children() {
              return createComponent(MemoryRouter, {
                history,
                root: App,
                get children() {
                  return createComponent(Route, {
                    path: "/*",
                    component: () => "Controlled Student route",
                  });
                },
              });
            },
          });
        },
      }),
      target,
    );
    return {
      dispose,
      navigate: (pathname) => history.set({ value: pathname }),
      pathname: history.get,
      releaseCourse: () => releaseCourse(course("CI7K3M2QAZ")),
      releaseAttempt: () => releaseAttempt({
        assessmentAttemptId: "00000000-0000-0000-0000-000000000001",
        attemptNumber: 1,
        displayTimeZone: "America/Chicago",
        expiresAt: null,
        timerRemainingMilliseconds: null,
        course: {
          id: "CI4W8QF9AD",
          shortName: "BIOL 302",
          longName: "Authorized Attempt course",
          theme: "ocean",
        },
        assessment: {
          id: "A9D2RX5AF",
          assessmentType: "regular_assignment",
          title: "Authorized Assessment",
        },
      }),
    };
  }
`;

async function bundleFixture() {
  const result = await build({
    bundle: true,
    format: "iife",
    globalName: "PleStudentCoursePinning",
    outfile: "student_course_pinning_fixture.js",
    platform: "browser",
    plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
    stdin: {
      contents: fixtureSource,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "student_course_pinning_fixture.tsx",
    },
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  const stylesheet = result.outputFiles.find((output) => output.path.endsWith(".css"));
  if (javascript === undefined)
    throw new Error("Student Course pinning fixture is missing JavaScript.");
  return { javascript: javascript.contents, stylesheet: stylesheet?.text ?? "" };
}

function control(page, id) {
  return page.locator(RIBBON_CONTROL_SELECTOR.replace("%s", id));
}

async function assertCourseLinks(page, courseInstanceId, stage) {
  const expected =
    courseInstanceId === undefined
      ? { coursework: "/student", grades: "/student" }
      : {
          coursework: `/student/courses/${courseInstanceId}`,
          grades: `/student/courses/${courseInstanceId}/grades`,
        };
  for (const [id, href] of Object.entries(expected)) {
    const link = control(page, id);
    await link.waitFor({ state: "visible" });
    assert.equal(await link.getAttribute("href"), href, `${stage}: ${id} pins its Course route`);
  }
}

async function assertResponsiveCourseLinks(page, courseInstanceId, stage) {
  for (const [viewportId, viewport] of Object.entries(CANONICAL_VIEWPORTS)) {
    await page.setViewportSize({ width: viewport.width, height: viewport.height });
    await assertCourseLinks(page, courseInstanceId, `${stage}/${viewportId}`);
    assert.equal(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
      true,
      `${stage}/${viewportId}: tier-one Student links do not create horizontal overflow`,
    );
  }
}

const fixture = await bundleFixture();
const css = [
  readFileSync(new URL("../../src/style.css", import.meta.url), "utf8"),
  readFileSync(new URL("../../src/style_responsive.css", import.meta.url), "utf8"),
  readFileSync(new URL("../../src/styles/accessibility.css", import.meta.url), "utf8"),
  fixture.stylesheet,
].join("\n");
const server = createServer((_request, response) => {
  response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
  response.end(
    `<!doctype html><html><head><style>${css}</style></head><body><div id="root"></div></body></html>`,
  );
});
server.listen(0, "127.0.0.1");
await once(server, "listening");
const address = server.address();
if (address === null || typeof address === "string")
  throw new Error("Student Course pinning fixture did not bind TCP.");

const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage({ viewport: CANONICAL_VIEWPORTS.laptop });
  const pageErrors = [];
  const consoleErrors = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  await page.goto(`http://127.0.0.1:${String(address.port)}/`);
  await page.addScriptTag({ content: Buffer.from(fixture.javascript).toString("utf8") });
  await page.evaluate(() => {
    const target = document.querySelector("#root");
    if (!(target instanceof HTMLElement))
      throw new Error("Student Course pinning root is missing.");
    window.studentCoursePinning =
      window.PleStudentCoursePinning.mountStudentCoursePinningHarness(target);
  });

  await page.evaluate(
    (pathname) => window.studentCoursePinning.navigate(pathname),
    `/student/courses/${COURSE_INSTANCE_ID}`,
  );
  await assertCourseLinks(page, undefined, "Course context before authorization resolves");
  await page.evaluate(() => window.studentCoursePinning.releaseCourse());
  await assertResponsiveCourseLinks(page, COURSE_INSTANCE_ID, "Resolved Course context");

  await page.evaluate(
    (pathname) => window.studentCoursePinning.navigate(pathname),
    `/assessment-attempts/${ASSESSMENT_ATTEMPT_ID}`,
  );
  await assertCourseLinks(page, COURSE_INSTANCE_ID, "Attempt context retains the pinned Course");
  await page.evaluate(() => window.studentCoursePinning.releaseAttempt());
  await assertResponsiveCourseLinks(page, ATTEMPT_COURSE_INSTANCE_ID, "Resolved Attempt context");

  await control(page, "grades").click();
  await page.waitForFunction(
    (expected) => window.studentCoursePinning.pathname() === expected,
    `/student/courses/${ATTEMPT_COURSE_INSTANCE_ID}/grades`,
  );
  await page.evaluate((pathname) => window.studentCoursePinning.navigate(pathname), "/sign-in");
  await page.evaluate(
    (pathname) => window.studentCoursePinning.navigate(pathname),
    `/student/courses/${COURSE_INSTANCE_ID}`,
  );
  await assertCourseLinks(page, undefined, "A new account starts its Course request unpinned");
  await page.evaluate(() => window.studentCoursePinning.releaseCourse());
  await assertResponsiveCourseLinks(page, COURSE_INSTANCE_ID, "New account Course context");
  assert.deepEqual(pageErrors, [], "Student Course pinning fixture raises no browser page errors");
  assert.deepEqual(
    consoleErrors,
    [],
    "Student Course pinning fixture raises no browser console errors",
  );
  process.stdout.write(
    "Student Coursework and Grades pin to resolved authorized Course contexts.\n",
  );
} finally {
  await browser.close();
  server.close();
}
