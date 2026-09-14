// Production-browser proof for explicit Blueprint Revision Saves and protected local editing.
// Selector contract:
// - src/pages/sign_in_page.tsx:89-128 and src/ribbon/ribbon_catalog.ts:256-275 own the
//   seeded Instructor entry and Blueprint Course navigation.
// - src/features/blueprint_course/blueprint_course_create_dialog.tsx:128-258 and
//   src/features/question_picker/question_picker.tsx:358-395 own creation and Question selection.
// - src/features/blueprint_course/blueprint_course_workspace.tsx:569-760 owns Revision Save,
//   metadata, archive/restore, and dirty-navigation controls.

import { chromium } from "playwright";

const port = process.argv[2];
if (!/^[0-9]+$/.test(port ?? "")) {
  throw new Error("expected the fixed HTTPS gateway port");
}

const origin = `https://localhost:${port}`;
const runId = Date.now();
const courseShortName = `Browser BP ${runId}`;
const courseLongName = `Browser Blueprint Course ${runId}`;
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();
const createPath = /^\/api\/course-blueprints$/u;

function blueprintSavePath(reference) {
  return new RegExp(`^/api/course-blueprints/${reference}$`, "u");
}

async function openAssignmentEditor(target) {
  await target.getByRole("button", { name: "Open Course Editor", exact: true }).click();
  await target.getByRole("button", { name: "Edit assignment", exact: true }).click();
}

async function expectBeforeUnload(target, action) {
  const warning = target.waitForEvent("dialog");
  const actionResult = action();
  const dialog = await warning;
  if (dialog.type() !== "beforeunload") {
    throw new Error(`expected a beforeunload warning, received ${dialog.type()}`);
  }
  await dialog.dismiss();
  await actionResult.catch(() => undefined);
}

try {
  await page.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
  await page
    .getByRole("button", { name: "Assume the role of Instructor Dr. Elena Rivera" })
    .click();
  await page.waitForURL(`${origin}/library`);
  await page
    .getByRole("navigation", { name: "Ribbon tabs", exact: true })
    .getByRole("link", { name: "Courses", exact: true })
    .click();
  await page.waitForURL(`${origin}/`);
  await page
    .getByRole("navigation", { name: "Ribbon tasks", exact: true })
    .getByRole("link", { name: "My Blueprint Courses", exact: true })
    .click();
  await page.waitForURL(`${origin}/blueprint-courses`);
  await page.getByRole("button", { name: "Create Blueprint Course" }).click();
  await page.getByRole("heading", { name: "Create a Blueprint Course" }).waitFor();
  await page.getByLabel("Blueprint Course short name").fill(courseShortName);
  await page.getByLabel("Blueprint Course long name").fill(courseLongName);
  const unsavedCreationHeading = page.getByRole("heading", {
    name: "Keep creating this Blueprint Course?",
    exact: true,
  });
  await page.getByRole("button", { name: "Close creation", exact: true }).click();
  await unsavedCreationHeading.waitFor();
  if ((await page.getByLabel("Blueprint Course short name").inputValue()) !== courseShortName) {
    throw new Error("Close creation discarded local Blueprint working state before confirmation");
  }
  await page.getByRole("button", { name: "Stay and keep editing", exact: true }).click();
  await page.keyboard.press("Escape");
  await unsavedCreationHeading.waitFor();
  if ((await page.getByLabel("Blueprint Course long name").inputValue()) !== courseLongName) {
    throw new Error("Escape discarded local Blueprint working state before confirmation");
  }
  await page.getByRole("button", { name: "Stay and keep editing", exact: true }).click();
  let createRequests = 0;
  page.on("request", (request) => {
    if (request.method() === "POST" && createPath.test(new URL(request.url()).pathname)) {
      createRequests += 1;
    }
  });
  await page.getByRole("dialog").getByRole("button", { name: "Create Blueprint Course" }).click();
  await page
    .getByText("Add at least one fixed Question or Question Pool before saving.", { exact: true })
    .waitFor();
  if (
    createRequests !== 0 ||
    (await page.getByLabel("Blueprint Course short name").inputValue()) !== courseShortName ||
    (await page.getByLabel("Blueprint Course long name").inputValue()) !== courseLongName
  ) {
    throw new Error("incomplete Blueprint Course creation wrote or discarded local working state");
  }
  await page.getByRole("button", { name: "Choose published Questions" }).click();
  await page.getByRole("heading", { name: "Choose the first reusable Questions" }).waitFor();
  await page.getByRole("button", { name: "Search questions" }).click();
  const firstQuestion = page
    .getByRole("dialog", { name: "Choose the first reusable Questions", exact: true })
    .getByRole("checkbox")
    .first();
  await firstQuestion.check();
  await page.getByRole("button", { name: "Use selected Questions" }).click();
  await page.getByText("1 fixed Question selected in order.").waitFor();
  const createdResponse = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" && createPath.test(new URL(response.url()).pathname),
  );
  await page.getByRole("dialog").getByRole("button", { name: "Create Blueprint Course" }).click();
  const creationResponse = await createdResponse;
  if (creationResponse.status() !== 201) {
    const body = (await creationResponse.text()).replaceAll(/\s+/gu, " ").slice(0, 1000);
    throw new Error(
      `complete Blueprint Course creation expected HTTP 201, received ${creationResponse.status()}: ${body}`,
    );
  }
  await page.waitForURL(/\/blueprint-courses\/BP-[1-9][0-9]*$/u);
  await page.getByRole("heading", { name: courseLongName }).waitFor();
  await page.getByText(/Current Revision 1\./u).waitFor();

  const discoveryContext = await browser.newContext({ ignoreHTTPSErrors: true });
  try {
    const discoveryPage = await discoveryContext.newPage();
    await discoveryPage.goto(`${origin}/sign-in`, { waitUntil: "domcontentloaded" });
    await discoveryPage
      .getByRole("button", { name: "Assume the role of Instructor Dr. Elena Rivera" })
      .click();
    await discoveryPage.waitForURL(`${origin}/library`);
    await discoveryPage
      .getByRole("navigation", { name: "Ribbon tabs", exact: true })
      .getByRole("link", { name: "Courses", exact: true })
      .click();
    await discoveryPage.waitForURL(`${origin}/`);
    await discoveryPage
      .getByRole("navigation", { name: "Ribbon tasks", exact: true })
      .getByRole("link", { name: "My Blueprint Courses", exact: true })
      .click();
    await discoveryPage.waitForURL(`${origin}/blueprint-courses`);
    await discoveryPage
      .getByRole("button", { name: "Create Blueprint Course", exact: true })
      .waitFor();
    const createdCourseLink = discoveryPage.getByRole("link", { name: courseLongName });
    for (
      let loadedPage = 0;
      loadedPage < 25 && (await createdCourseLink.count()) === 0;
      loadedPage += 1
    ) {
      const loadMore = discoveryPage.getByRole("button", {
        name: "Load more Blueprint Courses",
        exact: true,
      });
      if ((await loadMore.count()) === 0) break;
      const continuedPage = discoveryPage.waitForResponse((response) => {
        const url = new URL(response.url());
        return (
          response.request().method() === "GET" &&
          url.pathname === "/api/course-blueprints" &&
          url.searchParams.has("cursor")
        );
      });
      await loadMore.click();
      if (!(await continuedPage).ok()) {
        throw new Error(
          "Blueprint Course pagination did not return a successful continuation page",
        );
      }
    }
    if ((await createdCourseLink.count()) === 0) {
      throw new Error("fresh Instructor session could not discover the created Blueprint Course");
    }
  } finally {
    await discoveryContext.close();
  }

  const reference = page.url().match(/\/blueprint-courses\/(BP-[1-9][0-9]*)$/u)?.[1];
  if (reference === undefined)
    throw new Error("Blueprint Course route did not expose its reference");
  const savePath = blueprintSavePath(reference);
  const metadataPath = new RegExp(`^/api/course-blueprints/${reference}/metadata$`, "u");
  const archivePath = new RegExp(`^/api/course-blueprints/${reference}/archive$`, "u");
  const restorePath = new RegExp(`^/api/course-blueprints/${reference}/restore$`, "u");
  await openAssignmentEditor(page);
  const save = page.getByRole("button", { name: "Save Blueprint Course", exact: true });
  if (!(await save.isDisabled())) throw new Error("Save was enabled for clean Revision 1 content");

  let saveRequests = 0;
  page.on("request", (request) => {
    if (request.method() === "PUT" && savePath.test(new URL(request.url()).pathname)) {
      saveRequests += 1;
    }
  });
  const assignmentTitle = page.getByLabel("Assignment title", { exact: true });
  await assignmentTitle.fill(`Unsaved first edit ${runId}`);
  await assignmentTitle.fill(`Saved second edit ${runId}`);
  if (saveRequests !== 0) throw new Error("local Blueprint edits wrote before explicit Save");
  if (await save.isDisabled()) throw new Error("Save stayed disabled after local Blueprint edits");
  const savedRevisionTwo = page.waitForResponse(
    (response) =>
      response.request().method() === "PUT" && savePath.test(new URL(response.url()).pathname),
  );
  await save.click();
  if ((await savedRevisionTwo).status() !== 200)
    throw new Error("changed Blueprint Save did not succeed");
  await page.getByText("Saved Blueprint Revision 2.", { exact: true }).waitFor();
  if (saveRequests !== 1 || !(await save.isDisabled())) {
    throw new Error("multiple local edits did not converge on one clean explicit Save");
  }

  const staleEditor = await context.newPage();
  let revisionThreeEtag;
  try {
    await staleEditor.goto(page.url(), { waitUntil: "domcontentloaded" });
    await staleEditor.getByRole("heading", { name: courseLongName }).waitFor();
    await openAssignmentEditor(staleEditor);
    const staleEditorTitle = staleEditor.getByLabel("Assignment title", { exact: true });
    await staleEditorTitle.fill(`Other editor revision ${runId}`);
    const savedRevisionThree = staleEditor.waitForResponse(
      (response) =>
        response.request().method() === "PUT" && savePath.test(new URL(response.url()).pathname),
    );
    await staleEditor.getByRole("button", { name: "Save Blueprint Course", exact: true }).click();
    const revisionThreeResponse = await savedRevisionThree;
    revisionThreeEtag = revisionThreeResponse.headers().etag;
    if (revisionThreeEtag !== '"3"') {
      throw new Error("changed Revision 3 Save did not return its exact Revision ETag");
    }
    await staleEditor.getByText("Saved Blueprint Revision 3.", { exact: true }).waitFor();

    const retainedTitle = `Typed stale editor content ${runId}`;
    await assignmentTitle.fill(retainedTitle);
    const staleSave = page.waitForResponse(
      (response) =>
        response.request().method() === "PUT" &&
        savePath.test(new URL(response.url()).pathname) &&
        response.status() === 412,
    );
    await save.click();
    await staleSave;
    await page
      .getByText("Blueprint Course content changed in another editor. Reload before saving.", {
        exact: true,
      })
      .waitFor();
    await page.getByText(/Current Revision 2\./u).waitFor();
    if ((await assignmentTitle.inputValue()) !== retainedTitle) {
      throw new Error("stale Blueprint Save discarded the first editor's typed content");
    }
    await page.getByRole("button", { name: "Reload current Revision", exact: true }).click();
    await page.getByText(/Current Revision 3\./u).waitFor();
  } finally {
    await staleEditor.close();
  }

  await page.getByText("Course names and availability", { exact: true }).click();
  const renamedShortName = `Renamed BP ${runId}`;
  const renamedLongName = `Renamed Blueprint Course ${runId}`;
  await page.getByLabel("Blueprint Course short name", { exact: true }).fill(renamedShortName);
  await page.getByLabel("Blueprint Course long name", { exact: true }).fill(renamedLongName);
  const renamed = page.waitForResponse(
    (response) =>
      response.request().method() === "PUT" && metadataPath.test(new URL(response.url()).pathname),
  );
  await page.getByRole("button", { name: "Save Blueprint Course names", exact: true }).click();
  const renamedResponse = await renamed;
  const renamedMetadataEtag = renamedResponse.headers().etag;
  if (renamedResponse.status() !== 200 || renamedMetadataEtag === revisionThreeEtag) {
    throw new Error("Blueprint Course rename did not use a separate opaque metadata ETag");
  }
  await page
    .getByText("Blueprint Course names saved. Revision content is unchanged.", { exact: true })
    .waitFor();
  await page.getByRole("heading", { name: renamedLongName, exact: true }).waitFor();
  await page.getByText(/Current Revision 3\./u).waitFor();
  const reloadedAfterRename = page.waitForResponse(
    (response) =>
      response.request().method() === "GET" && savePath.test(new URL(response.url()).pathname),
  );
  await page.reload({ waitUntil: "domcontentloaded" });
  if ((await reloadedAfterRename).headers().etag !== revisionThreeEtag) {
    throw new Error("renaming Blueprint Course names changed the current Revision ETag");
  }
  await page.getByRole("heading", { name: renamedLongName, exact: true }).waitFor();
  await page.getByText(/Current Revision 3\./u).waitFor();

  await openAssignmentEditor(page);
  await page.getByText("Course names and availability", { exact: true }).click();
  await page
    .getByLabel("Confirm Blueprint Course long name", { exact: true })
    .fill(renamedLongName);
  const archived = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" && archivePath.test(new URL(response.url()).pathname),
  );
  await page.getByRole("button", { name: "Archive Blueprint Course", exact: true }).click();
  const archivedResponse = await archived;
  const archivedMetadataEtag = archivedResponse.headers().etag;
  if (
    archivedResponse.status() !== 200 ||
    archivedMetadataEtag === renamedMetadataEtag ||
    archivedMetadataEtag === revisionThreeEtag
  ) {
    throw new Error("Blueprint Course archive did not advance only its opaque metadata ETag");
  }
  await page
    .getByText("Blueprint Course archived. Its saved Revisions are unchanged.", { exact: true })
    .waitFor();
  await page.getByText(/Current Revision 3\./u).waitFor();
  await page.getByRole("button", { name: "Restore Blueprint Course", exact: true }).waitFor();

  const restored = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" && restorePath.test(new URL(response.url()).pathname),
  );
  await page.getByRole("button", { name: "Restore Blueprint Course", exact: true }).click();
  const restoredResponse = await restored;
  if (
    restoredResponse.status() !== 200 ||
    restoredResponse.headers().etag === archivedMetadataEtag
  ) {
    throw new Error("Blueprint Course restore did not use the next opaque metadata ETag");
  }
  await page
    .getByText("Blueprint Course restored. Its saved Revisions are unchanged.", { exact: true })
    .waitFor();
  await page.getByText(/Current Revision 3\./u).waitFor();

  await assignmentTitle.fill(`Router guard ${runId}`);
  await page.getByRole("link", { name: "Return to Blueprint Courses", exact: true }).click();
  await page
    .getByRole("heading", { name: "Save Blueprint Course changes?", exact: true })
    .waitFor();
  await page.getByRole("button", { name: "Stay and keep editing", exact: true }).click();
  if ((await assignmentTitle.inputValue()) !== `Router guard ${runId}`) {
    throw new Error("Stay did not retain local Blueprint Course edits");
  }

  await page.goBack();
  await page
    .getByRole("heading", { name: "Save Blueprint Course changes?", exact: true })
    .waitFor();
  await page.getByRole("button", { name: "Discard and continue", exact: true }).click();
  await page.waitForURL(`${origin}/blueprint-courses`);
  await page.getByRole("link", { name: renamedLongName }).waitFor();

  await page.goForward();
  await page.waitForURL(new RegExp(`/blueprint-courses/${reference}$`, "u"));
  await openAssignmentEditor(page);
  await assignmentTitle.fill(`Reload guard ${runId}`);
  await expectBeforeUnload(page, () => page.reload({ waitUntil: "domcontentloaded" }));
  if ((await assignmentTitle.inputValue()) !== `Reload guard ${runId}`) {
    throw new Error("beforeunload reload protection discarded typed Blueprint Course content");
  }

  const closeGuardPage = await context.newPage();
  await closeGuardPage.goto(page.url(), { waitUntil: "domcontentloaded" });
  await openAssignmentEditor(closeGuardPage);
  await closeGuardPage.getByLabel("Assignment title", { exact: true }).fill(`Close guard ${runId}`);
  await expectBeforeUnload(closeGuardPage, () => closeGuardPage.close({ runBeforeUnload: true }));
  if (closeGuardPage.isClosed()) {
    throw new Error("beforeunload page-close protection allowed dirty Blueprint editor closure");
  }
  await closeGuardPage.close();
} finally {
  await context.close();
  await browser.close();
}
