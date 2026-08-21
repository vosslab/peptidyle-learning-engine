// Opt-in simulated-data capture of every instructor-visible work surface.

import { expect, test, type Page, type Route } from "@playwright/test";

import type { GradebookSummaryRow } from "../../generated/api/GradebookSummaryRow";
import { publishedProblemFixture } from "../../generated/fixtures/published_problem";
import {
  captureDocumentationScreenshot,
  type DocumentationScreenshotName,
} from "./docs_screenshot_capture";

const outputDirectory = process.env["PLE_INSTRUCTOR_PAGE_VISUALS_DIR"];
const courseId = publishedProblemFixture.course.id;
const courseReference = publishedProblemFixture.course.reference;
const assignmentId = publishedProblemFixture.assignment.id;
const assignmentReference = publishedProblemFixture.assignment.reference;
const workspaceId = publishedProblemFixture.draft.workspace;
const workspaceReference = "W-1";

function fixtureUuid(value: number): string {
  return `0198e000-0000-7000-8000-${value.toString().padStart(12, "0")}`;
}

const course = {
  ...publishedProblemFixture.course,
  title: "BIOC 301: Biochemistry",
  role: "instructor" as const,
};

const courses = [
  course,
  {
    ...course,
    id: fixtureUuid(210),
    reference: "A-2",
    title: "GEN 220: Genetics",
  },
  {
    ...course,
    id: fixtureUuid(211),
    reference: "A-3",
    title: "MOLB 330: Molecular Biology",
  },
] as const;

const catalogProblems = [
  {
    ...publishedProblemFixture.catalogProblem,
    metadata: {
      ...publishedProblemFixture.catalogProblem.metadata,
      title: "Peptide bond resonance and planarity",
    },
  },
  {
    ...publishedProblemFixture.catalogProblem,
    questionId: "ABC-123T",
    metadata: {
      ...publishedProblemFixture.catalogProblem.metadata,
      title: "Ramachandran angle interpretation",
      tags: ["biochemistry", "protein-folding"],
    },
  },
  {
    ...publishedProblemFixture.catalogProblem,
    questionId: "PEP-T1D3",
    metadata: {
      ...publishedProblemFixture.catalogProblem.metadata,
      title: "Protein folding energy landscape",
      tags: ["biochemistry", "thermodynamics"],
    },
  },
  {
    ...publishedProblemFixture.catalogProblem,
    questionId: "GEN-E42K",
    metadata: {
      ...publishedProblemFixture.catalogProblem.metadata,
      title: "Enzyme active-site geometry",
      tags: ["biochemistry", "enzymes"],
    },
  },
] as const;

function assignmentItems(
  offset: number,
  count: number,
): typeof publishedProblemFixture.assignment.items {
  return catalogProblems.slice(0, count).map((problem, index) => ({
    id: fixtureUuid(offset + index),
    questionId: problem.questionId,
    title: problem.metadata.title,
    backend: problem.backend,
    capabilities: problem.capabilities,
    position: index,
    pointsPossible: "1",
    deliveryState: "active" as const,
    scoringMode: "normal" as const,
  }));
}

const assignments = [
  {
    ...publishedProblemFixture.assignment,
    title: "Biochemistry structure practice",
    items: assignmentItems(230, 4),
  },
  {
    ...publishedProblemFixture.assignment,
    id: fixtureUuid(240),
    reference: "C-2",
    title: "Enzyme kinetics check-in",
    items: assignmentItems(250, 3),
  },
  {
    ...publishedProblemFixture.assignment,
    id: fixtureUuid(260),
    reference: "C-3",
    title: "Protein folding review",
    items: assignmentItems(270, 4),
  },
] as const;

const assignmentEditor = {
  ...assignments[0],
  teachingSettings: {
    timeZone: "America/Chicago",
    lifecycle: "published",
    instructions: "",
    availableAt: null,
    dueAt: null,
    closesAt: null,
    timeLimitSeconds: 900,
    attemptLimit: null,
    lateSubmission: "accept",
    deadlineBehavior: "autoSubmit",
  },
  currentState: { state: "open" },
};

const fixtureGradebookRow = publishedProblemFixture.gradebook[0];
if (fixtureGradebookRow === undefined) {
  throw new Error("The instructor demo corpus needs one gradebook source row.");
}

function gradebookRow(
  learnerName: "Mary Fake Student" | "Jack Fake Student",
  assignmentIndex: 0 | 1,
  sequence: number,
  best: number,
  latest: number,
): GradebookSummaryRow {
  const enrollmentId = fixtureUuid(300 + sequence);
  const assignment = assignments[assignmentIndex];
  return {
    ...fixtureGradebookRow,
    tenant: course.tenant,
    courseId,
    enrollmentId,
    studentId: fixtureUuid(320 + sequence),
    learnerName,
    assignmentId: assignment.id,
    assignmentTitle: assignment.title,
    scoringStatus: "current",
    summary: {
      ...publishedProblemFixture.summary,
      enrollment: enrollmentId,
      currentScore: best,
      bestScore: best,
      latestScore: latest,
      completedRunCount: sequence + 2,
      totalQuestionAttempts: (sequence + 2) * 4,
      lastActivityAt: 1_786_000_004_100 - sequence * 3_600_000,
    },
  };
}

const gradebook = [
  gradebookRow("Mary Fake Student", 0, 0, 1, 1),
  gradebookRow("Jack Fake Student", 0, 1, 0.75, 0.5),
  gradebookRow("Mary Fake Student", 1, 2, 0.9, 0.9),
  gradebookRow("Jack Fake Student", 1, 3, 0.8, 0.8),
];

const laboratoryCategoryId = fixtureUuid(370);
const examinationCategoryId = fixtureUuid(371);
const courseGradeScheme = {
  scheme: {
    mode: "weightedCategories",
    rounding: "fourDecimalPlacesHalfAwayFromZero",
    categories: [
      {
        id: laboratoryCategoryId,
        title: "Laboratory practice",
        position: 0,
        weightBasisPoints: 4_000,
        dropLowest: 1,
      },
      {
        id: examinationCategoryId,
        title: "Examinations",
        position: 1,
        weightBasisPoints: 6_000,
        dropLowest: 0,
      },
    ],
    letterBands: [
      { label: "A", minimumBasisPoints: 9_000 },
      { label: "B", minimumBasisPoints: 8_000 },
      { label: "C", minimumBasisPoints: 7_000 },
    ],
  },
  assignments: assignments.map((assignment, position) => ({
    assignment: assignment.id,
    title: assignment.title,
    included: true,
    category: position < 2 ? laboratoryCategoryId : examinationCategoryId,
    position: position < 2 ? position : 0,
  })),
} as const;

const courseGradeTotals = {
  mode: "weightedCategories",
  rounding: "fourDecimalPlacesHalfAwayFromZero",
  rows: [
    {
      rosterId: "BIO-1042",
      displayName: "Mary Fake Student",
      outcome: {
        status: "available",
        score: 0.94,
        letter: "A",
        droppedAssignmentIds: [assignments[1].id],
        totalEarned: null,
        totalPossible: null,
      },
    },
    {
      rosterId: "BIO-1043",
      displayName: "Jack Fake Student",
      outcome: { status: "unavailable", reason: "recalculating" },
    },
  ],
} as const;

const roster = {
  rosterMode: "emailEnrollment",
  members: [
    {
      memberId: fixtureUuid(340),
      displayName: "Mary Fake Student",
      rosterEmail: "mary.fake@students.example.invalid",
      rosterId: "BIO-1042",
      role: "student",
      status: "active",
    },
    {
      memberId: fixtureUuid(341),
      displayName: "Jack Fake Student",
      rosterEmail: "jack.fake@students.example.invalid",
      rosterId: "BIO-1043",
      role: "student",
      status: "active",
    },
  ],
  pendingInvitations: [
    {
      invitationId: fixtureUuid(342),
      email: "new.fake@students.example.invalid",
      rosterId: "BIO-1044",
      status: "pending",
      expiresAt: 1_791_000_000_000,
    },
  ],
  allowedEmailDomains: [{ domain: "students.example.invalid", includeSubdomains: false }],
  signupPosture: "invitationOnly",
  nextCursor: null,
  rosterRevision: 4,
} as const;

const workspaceDrafts = [
  {
    workspace: workspaceId,
    reference: "W-1",
    title: "Peptide bond resonance revision",
    sourceBackend: "native",
  },
  {
    workspace: fixtureUuid(350),
    reference: "W-2",
    title: "DNA replication checkpoint",
    sourceBackend: "native",
  },
  {
    workspace: fixtureUuid(351),
    reference: "W-3",
    title: "Enzyme inhibition comparison",
    sourceBackend: "native",
  },
] as const;

const flatQuestionSource = {
  format: "pleFlatQuestion",
  version: 2,
  title: "Peptide bond concept check",
  prompt: "Which bond has partial double-bond character in a peptide group?",
  response: {
    kind: "singleChoice",
    choices: [
      { id: "amide", text: "The carbonyl carbon-to-nitrogen bond", feedback: null },
      { id: "carbonyl", text: "The carbonyl carbon-to-oxygen bond", feedback: null },
      { id: "alpha", text: "The nitrogen-to-alpha-carbon bond", feedback: null },
    ],
    correctChoice: "amide",
  },
  feedback: {
    correct: "Yes. Resonance restricts rotation around the peptide C-N bond.",
    incorrect: "Look for the bond whose electrons are shared with the carbonyl group.",
  },
  points: 1,
  attemptPolicy: { maxAttempts: null },
  timingPolicy: { kind: "untimed" },
  tags: ["biochemistry", "protein-structure"],
  taxonomy: [],
  license: { kind: "ccBySa" },
  language: "en-US",
} as const;

function json(
  route: Route,
  value: unknown,
  status = 200,
  headers: Readonly<Record<string, string>> = {},
): Promise<void> {
  return route.fulfill({
    status,
    headers: { "content-type": "application/json", ...headers },
    body: JSON.stringify(value),
  });
}

async function installSimulatedInstructorApi(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
      set: () => undefined,
    });
  });
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const path = url.pathname;
    const method = request.method();
    if (path === "/api/auth/session") {
      return await json(route, {
        authenticated: true,
        tenant: course.tenant,
        user: {
          id: fixtureUuid(360),
          displayName: "Dr. Fake Professor",
          roles: ["instructor"],
        },
      });
    }
    if (path === "/api/auth/account/presentation") {
      return await json(route, { contrast: "standard" });
    }
    if (path === "/api/auth/passkeys") {
      return await json(route, [
        {
          id: fixtureUuid(361),
          label: "Biology laptop",
          createdAtMillis: 1_780_000_000_000,
          lastUsedAtMillis: 1_786_000_000_000,
        },
        {
          id: fixtureUuid(362),
          label: "Office security key",
          createdAtMillis: 1_770_000_000_000,
          lastUsedAtMillis: null,
        },
      ]);
    }
    if (path === "/api/courses") {
      return await json(route, { items: courses, nextCursor: null });
    }
    if (path === `/api/navigation/${courseReference}`) {
      return await json(route, { kind: "course", courseId });
    }
    if (path === `/api/navigation/${assignmentReference}`) {
      return await json(route, { kind: "assignment", courseId, assignmentId });
    }
    if (path === `/api/navigation/${workspaceReference}`) {
      return await json(route, { kind: "workspace", workspaceId });
    }
    if (path === `/api/courses/${courseId}`) return await json(route, course);
    if (path === `/api/courses/${courseId}/appearance`) {
      return await json(route, { theme: "grass", revision: "1", banner: null }, 200, {
        "cache-control": "no-store",
        etag: '"1"',
      });
    }
    if (path === `/api/courses/${courseId}/assignments`) {
      return await json(route, { items: assignments, nextCursor: null });
    }
    if (path === `/api/assignments/${assignmentId}`) {
      const editing = new URL(page.url()).pathname.endsWith("/edit");
      return await json(route, editing ? assignmentEditor : assignments[0], 200, {
        etag: '"7"',
      });
    }
    if (path === `/api/courses/${courseId}/gradebook`) {
      return await json(route, { items: gradebook, nextCursor: null });
    }
    if (path === `/api/courses/${courseId}/grade-scheme`) {
      return await json(route, courseGradeScheme, 200, {
        "cache-control": "no-store",
        etag: '"4"',
      });
    }
    if (path === `/api/courses/${courseId}/gradebook-totals`) {
      return await json(route, courseGradeTotals, 200, { "cache-control": "no-store" });
    }
    if (path === `/api/courses/${courseId}/roster`) return await json(route, roster);
    if (path === "/api/problems/search") {
      return await json(route, {
        items: catalogProblems,
        nextCursor: null,
        facets: {
          taxonomy: [
            {
              term: {
                scheme: "Peptidyle",
                code: "BIOCHEM.PEPTIDE_BOND",
                label: "Peptide bond structure",
              },
              count: 4,
            },
          ],
          capabilities: [
            { capability: "algorithmicGeneration", count: 4 },
            { capability: "serverGrading", count: 4 },
          ],
          licenses: [{ license: "ccBy", count: 4 }],
          statistics: { available: 0, unavailable: 4 },
        },
      });
    }
    const detail = catalogProblems.find(
      (problem) => path === `/api/problems/by-id/${problem.questionId}/detail`,
    );
    if (detail !== undefined) {
      return await json(route, {
        summary: detail,
        prompt: publishedProblemFixture.publishedProblem.prompt,
        statistics: "unavailable",
      });
    }
    if (path.startsWith("/api/problems/by-id/")) {
      const displayId = decodeURIComponent(path.slice("/api/problems/by-id/".length)).toUpperCase();
      const problem = catalogProblems.find((candidate) => candidate.questionId === displayId);
      return problem === undefined
        ? await json(route, { error: "question not found" }, 404)
        : await json(route, problem);
    }
    if (path === "/api/workspaces") {
      return await json(route, { items: workspaceDrafts, nextCursor: null });
    }
    if (path === `/api/workspaces/${workspaceId}/flat-question-assets`) {
      return await json(route, []);
    }
    if (path === `/api/workspaces/${workspaceId}/flat-question` && method === "GET") {
      return await route.fulfill({
        status: 200,
        headers: {
          "content-type": "application/vnd.peptidyle.flat-question+json",
          etag: '"7"',
        },
        body: JSON.stringify(flatQuestionSource),
      });
    }
    if (path === `/api/workspaces/${workspaceId}` && method === "GET") {
      return await json(route, publishedProblemFixture.draft, 200, { etag: '"7"' });
    }
    if (path.startsWith("/api/assets/") && path.endsWith("/delivery")) {
      const assetPath = path.slice(0, -"/delivery".length);
      return await json(route, { url: `${url.origin}${assetPath}` }, 200, {
        "cache-control": "no-store",
      });
    }
    if (path.startsWith("/api/assets/")) {
      return await route.fulfill({
        status: 200,
        contentType: "image/svg+xml",
        body: '<svg xmlns="http://www.w3.org/2000/svg" width="480" height="180" viewBox="0 0 480 180"><rect width="480" height="180" fill="#eef5e9"/><path d="M70 120 L180 80 L280 110 L410 55" fill="none" stroke="#006b44" stroke-width="10" stroke-linecap="round"/><circle cx="180" cy="80" r="18" fill="#73c167"/><circle cx="280" cy="110" r="18" fill="#f0b24d"/></svg>',
      });
    }
    return await json(route, { error: `Unexpected simulated request: ${method} ${path}` }, 500);
  });
}

async function navigate(page: Page, path: string, surface: string): Promise<void> {
  await page.evaluate((nextPath) => {
    history.pushState({}, "", nextPath);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, path);
  await expect(page.locator(`[data-route-surface="${surface}"]`)).toBeVisible();
}

async function capture(
  page: Page,
  name: DocumentationScreenshotName,
  anchor?: ReturnType<Page["locator"]>,
): Promise<void> {
  const exposure = await page.evaluate(async () => {
    await document.fonts.ready;
    if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
    const uuid = document.body.innerText.match(
      /[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}/iu,
    );
    return { pathname: window.location.pathname, uuid: uuid?.[0] ?? null };
  });
  expect(exposure.uuid, `${name} must not expose an internal UUID`).toBeNull();
  expect(exposure.pathname, `${name} must use a human-facing browser route`).not.toMatch(
    /[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}/iu,
  );
  await captureDocumentationScreenshot(page, name, anchor, undefined, outputDirectory);
}

test("captures the instructor demo-environment page corpus", async ({ page }) => {
  test.skip(outputDirectory === undefined, "requires the dedicated instructor visual launcher");
  if (outputDirectory === undefined) return;
  test.setTimeout(120_000);
  await page.setViewportSize({ width: 1_280, height: 800 });
  await installSimulatedInstructorApi(page);
  await page.goto("/");

  await expect(page.locator('[data-route-surface="courses"]')).toBeVisible();
  await capture(page, "instructor_page_courses.png");

  await navigate(page, `/courses/${courseReference}`, "courseAssignments");
  await capture(page, "instructor_page_course_assignments.png");

  await navigate(
    page,
    `/courses/${courseReference}/assignments/${assignmentReference}`,
    "assignmentOverview",
  );
  await capture(page, "instructor_page_assignment_overview.png");

  await navigate(
    page,
    `/instructor/courses/${courseReference}/assignments/new`,
    "assignmentEditor",
  );
  await capture(page, "instructor_page_assignment_create.png");

  await navigate(
    page,
    `/instructor/courses/${courseReference}/assignments/${assignmentReference}/edit`,
    "assignmentEditor",
  );
  await expect(page.locator(".assignment-editor-row")).toHaveCount(4);
  await capture(page, "instructor_page_assignment_edit.png");

  await navigate(page, `/instructor/courses/${courseReference}/students`, "courseRoster");
  await capture(page, "instructor_page_roster.png");

  await navigate(page, `/instructor/courses/${courseReference}/gradebook`, "gradebook");
  await expect(page.getByRole("table")).toBeVisible();
  await capture(page, "instructor_page_gradebook.png");

  await navigate(
    page,
    `/instructor/courses/${courseReference}/grade-settings`,
    "courseGradeSettings",
  );
  await expect(page.getByRole("radio", { name: "Weighted categories" })).toBeChecked();
  await expect(page.getByText("Weight total: 100.00% of 100.00%.")).toBeVisible();
  for (const name of [
    "course_grade_settings_laptop.png",
    "course_grade_settings_tablet.png",
    "course_grade_settings_iphone_pro.png",
    "course_grade_settings_square.png",
  ]) {
    await capture(page, name);
  }

  await navigate(page, `/instructor/courses/${courseReference}/appearance`, "courseAppearance");
  await capture(page, "instructor_page_course_appearance.png");

  await navigate(page, "/library", "library");
  await expect(page.locator(".catalog-row")).toHaveCount(4);
  await capture(page, "instructor_page_library.png");

  await navigate(page, `/library/${catalogProblems[0].questionId}`, "problemDetail");
  await expect(
    page.getByRole("heading", { name: catalogProblems[0].metadata.title }),
  ).toBeVisible();
  await capture(page, "instructor_page_question_detail.png");

  await navigate(page, "/workspace", "workspaceEditor");
  await expect(page.getByRole("heading", { name: "Your drafts", exact: true })).toBeVisible();
  await capture(page, "instructor_page_workspace.png");

  await navigate(page, `/workspace/${workspaceReference}`, "flatQuestionEditor");
  await expect(page.getByRole("heading", { name: "Flat question", exact: true })).toBeVisible();
  await capture(page, "instructor_page_question_editor.png");

  await navigate(page, "/account/security", "accountSecurity");
  await expect(page.getByText("Biology laptop", { exact: true })).toBeVisible();
  await capture(page, "instructor_page_account_security.png");
});
