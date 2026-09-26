import assert from "node:assert/strict";
import { after, test } from "node:test";

import { chromium } from "playwright";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

import { startHarnessServer } from "./ribbon_harness_server.mjs";

// User-facing selectors follow CourseInstancePage's Assessment actions, inline editor, and facts.
const bundledPage = await build({
  stdin: {
    contents: `
      import { MemoryRouter, Route, createMemoryHistory } from "@solidjs/router";
      import { createComponent } from "solid-js";
      import { render } from "solid-js/web";

      import { ApplicationApiProvider, createApplicationApi } from "../../src/api/application_api";
      import { LiveAssessmentWorkspaceConflictError } from "../../src/api/http_client/assessment_release";
      import { CourseInstancePage } from "../../src/pages/course_instance_page";
      import { RouteScopeProvider } from "../../src/ribbon/route_scope_context";

      const courseInstanceId = "CI7K3M2QAZ";
      const initialAssessment = {
        id: "A1",
        assessmentType: "regular_assignment",
        title: "Original Assessment",
        dueAt: "2026-10-01T09:00:00",
        displayTimeZone: "America/Chicago",
        status: "unreleased",
        assessmentEditNumber: "4",
      };
      const currentAssessment = {
        ...initialAssessment,
        title: "Current Assessment",
        dueAt: "2026-10-02T10:30:00",
        status: "released",
        assessmentEditNumber: "5",
      };
      let assessmentLoads = 0;
      let saveAttempts = 0;
      const client = {
        getCourseInstance: () => Promise.resolve({
          courseInstance: {
            id: courseInstanceId,
            shortName: "BIO 301",
            longName: "Biochemistry",
            classification: {
              disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
              subjectUuid: null,
              topicUuid: null,
              subtopicUuid: null,
              tags: [],
            },
            lifecycleState: "active",
            courseEditNumber: "1",
            term: { startDate: "2026-08-24", endDate: "2026-12-12" },
            theme: "grass",
          },
          activeInstructorCount: 1,
          blueprintOrigin: null,
        }),
        listCourseAssessments: () => Promise.resolve([
          assessmentLoads++ === 0 ? initialAssessment : currentAssessment,
        ]),
        getProfile: () => new Promise(() => undefined),
        saveLiveAssessmentInline: (_courseId, _assessmentId, input) => {
          saveAttempts += 1;
          if (saveAttempts === 1) {
            return Promise.reject(new LiveAssessmentWorkspaceConflictError("/assessments/A1"));
          }
          return Promise.resolve({
            ...currentAssessment,
            title: input.title,
            dueAt: input.dueAt,
            assessmentEditNumber: "6",
          });
        },
      };

      export function mountCourseInstancePage() {
        const applicationApi = createApplicationApi(client);
        const history = createMemoryHistory();
        history.set({ value: "/instructor/courses/" + courseInstanceId });
        return render(
          () => createComponent(ApplicationApiProvider, {
            applicationApi,
            get children() {
              return createComponent(RouteScopeProvider, {
                pathname: "/instructor/courses/" + courseInstanceId,
                get children() {
                  return createComponent(MemoryRouter, {
                    history,
                    get children() {
                      return createComponent(Route, {
                        path: "/instructor/courses/:courseInstanceId",
                        component: CourseInstancePage,
                      });
                    },
                  });
                },
              });
            },
          }),
          document.body,
        );
      }

    `,
    resolveDir: new URL(".", import.meta.url).pathname,
    sourcefile: "course_instance_assessment_refresh_page.tsx",
    loader: "tsx",
  },
  bundle: true,
  format: "esm",
  platform: "browser",
  write: false,
  loader: { ".css": "empty" },
  plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
});
const harnessServer = await startHarnessServer(
  `<!doctype html><html><body><script type="module">
    import { mountCourseInstancePage } from "/page.js";
    mountCourseInstancePage();
  </script></body></html>`,
  "",
  new Map([
    [
      "/page.js",
      {
        body: bundledPage.outputFiles[0].contents,
        contentType: "text/javascript; charset=utf-8",
      },
    ],
  ]),
);

test("Course Instance Assessment editing preserves current work through refresh, cancel, and conflict", async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    await page.goto(harnessServer.evidenceUrl);
    await page.getByRole("button", { name: "Edit title and due date" }).click();

    const titleInput = page.getByRole("textbox", { name: "Title" });
    await titleInput.fill("My typed draft");
    await page.getByRole("textbox", { name: "Due date" }).fill("2026-11-15");
    await page.getByRole("textbox", { name: "Due time" }).fill("14:45");
    await titleInput.focus();
    await page.getByRole("button", { name: "Save title and due date" }).click();
    await page
      .getByRole("alert")
      .getByText(/changed elsewhere/u)
      .waitFor();
    await page.getByRole("button", { name: "Load current Assessment" }).click();
    await page.getByText("Current Assessment", { exact: true }).waitFor();
    const currentRow = page.getByRole("listitem").filter({
      has: page.getByRole("heading", { name: "Current Assessment", exact: true }),
    });
    await currentRow.getByText("Released", { exact: true }).waitFor({ state: "visible" });
    const currentDue = currentRow.locator('time[datetime="2026-10-02T10:30:00"]');
    await currentDue.waitFor({ state: "visible" });
    assert.equal(await currentDue.innerText(), "Oct 2, 2026, 10:30 AM");
    assert.equal(await titleInput.inputValue(), "My typed draft");
    assert.equal(await page.getByRole("textbox", { name: "Due date" }).inputValue(), "2026-11-15");
    assert.equal(await page.getByRole("textbox", { name: "Due time" }).inputValue(), "14:45");
    assert.equal(await titleInput.evaluate((input) => input.matches(":focus")), true);

    await page.getByRole("button", { name: "Save title and due date" }).click();
    await page.getByRole("status").getByText("Assessment title and due date saved.").waitFor();

    const editButton = page.getByRole("button", { name: "Edit title and due date" });
    await editButton.click();
    await titleInput.fill("Discarded draft");
    await page.getByRole("button", { name: "Cancel" }).click();
    assert.equal(await editButton.evaluate((button) => button.matches(":focus")), true);
    await editButton.click();
    assert.equal(await titleInput.inputValue(), "My typed draft");
  } finally {
    await browser.close();
  }
});

after(async () => {
  await harnessServer.close();
});
