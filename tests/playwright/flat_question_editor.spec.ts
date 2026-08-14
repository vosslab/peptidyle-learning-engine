// flat_question_editor.spec.ts - visible acceptance for protected flat-question authoring.
// Selector contract: the native labels and buttons in flat_question_editor_page.tsx and its
// child fields are the instructor's actual keyboard-accessible editing surface.

import { expect, test } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

let fixtureScript = "";

test.beforeAll(async () => {
  const result = await build({
    bundle: true,
    format: "iife",
    minify: false,
    plugins: [solidPlugin()],
    platform: "browser",
    stdin: {
      contents: `
        import { createComponent } from "solid-js";
        import { render } from "solid-js/web";
        import { FlatQuestionEditorPage } from "./src/features/flat_question_authoring/flat_question_editor_page.tsx";
        import { createFlatQuestionClient } from "./src/features/flat_question_authoring/flat_question_client.ts";
        import { createFlatQuestionAssetClient } from "./src/features/flat_question_authoring/flat_question_asset_client.ts";
        import { createFlatQuestionRepository } from "./src/features/flat_question_authoring/flat_question_repository.ts";
        import { serializeFlatQuestionSource } from "./src/features/flat_question_authoring/flat_question_codec.ts";

        const workspace = "00000000-0000-4000-8000-000000000010";
        const problem = "00000000-0000-4000-8000-000000000011";
        const version = "00000000-0000-4000-8000-000000000012";
        const calls = [];
        const publicCalls = [];
        const hotspotAsset = { assetId: "aaaaaaaa-0000-4000-8000-000000000013", contentChecksum: "a".repeat(64), displayLabel: "Cell membrane diagram", mediaType: "image/png", intrinsicWidth: 800, intrinsicHeight: 600 };
        let revision = 1;
        let source = {
          format: "pleFlatQuestion", version: 2,
          title: "Favorite color", prompt: "What is my favorite color?",
          response: { kind: "singleChoice", choices: [
            { id: "blue", text: "Blue", feedback: null },
            { id: "red", text: "Red", feedback: null },
          ], correctChoice: "blue" }, feedback: { correct: null, incorrect: null }, points: 1,
          attemptPolicy: { maxAttempts: null, feedback: "immediateFull" },
          timingPolicy: { kind: "untimed" }, tags: ["example"], taxonomy: [],
          license: { kind: "ccBySa" }, language: "en-US",
        };
        let staleNextSave = false;
        let delaySave = false;
        let releaseSave = null;
        let delayValidation = false;
        let releaseValidation = null;
        let dispose = null;
        const json = (value, status = 200, headers = {}) => new Response(JSON.stringify(value), {
          status, headers: { "content-type": "application/json", ...headers },
        });
        const family = (kind) => ({
          singleChoice: "flat_single_choice_v2", multipleAnswer: "flat_multiple_answer_v2",
          fillIn: "flat_fill_in_v2", multiFillIn: "flat_multi_fill_in_v2", numeric: "flat_numeric_v2",
          matching: "flat_matching_v2", ordering: "flat_ordering_v2", hotspot: "flat_hotspot_v2",
        })[kind];
        const option = (item) => ({ id: item.id, body: [{ kind: "text", markdown: item.text }] });
        const publicResponse = (response) => {
          if (response.kind === "singleChoice") return { kind: "multipleChoice", choices: response.choices.map(option), selection: { kind: "exactlyOne" } };
          if (response.kind === "multipleAnswer") return { kind: "multipleChoice", choices: response.choices.map(option), selection: { kind: "atLeastOne" } };
          if (response.kind === "fillIn") return { kind: "shortText", matchMode: response.matchMode, maxLength: response.maxLength };
          if (response.kind === "multiFillIn") return { kind: "multiBlank", blanks: response.blanks.map((blank) => ({ id: blank.id, label: [{ kind: "text", markdown: blank.label }], matchMode: blank.matchMode, maxLength: blank.maxLength })) };
          if (response.kind === "numeric") return { kind: "numeric", tolerance: response.tolerance, unit: response.unit };
          if (response.kind === "matching") return { kind: "matching", prompts: response.prompts.map(option), choices: response.choices.map(option) };
          if (response.kind === "ordering") return { kind: "ordering", items: response.items.map(option) };
          return { kind: "hotspot", surface: { asset: response.surface.asset, checksum: response.surface.checksum }, description: response.surface.description, regions: response.regions.map((region) => ({ ...region, label: [{ kind: "text", markdown: region.label }] })), selection: { kind: "atLeastOne" } };
        };
        const compiled = (candidate, published = false) => ({
          ...(published ? { problem, version } : {}), workspace,
          source: { backend: "native", family: family(candidate.response.kind) },
          prompt: [{ kind: "text", markdown: candidate.prompt }],
          response: publicResponse(candidate.response),
          attemptPolicy: candidate.attemptPolicy, timingPolicy: candidate.timingPolicy,
          randomization: { kind: "static" }, grading: { mode: "allOrNothing", points: candidate.points },
          metadata: { title: candidate.title, tags: candidate.tags, taxonomy: candidate.taxonomy,
            license: candidate.license, language: candidate.language },
        });
        const transport = async (input, init = {}) => {
          const url = new URL(String(input), window.location.origin);
          const method = init.method ?? "GET";
          const body = typeof init.body === "string" ? init.body : null;
          const headers = new Headers(init.headers);
          calls.push({ method, path: url.pathname, body, ifMatch: headers.get("if-match") });
          const sourcePath = "/api/workspaces/" + workspace + "/flat-question";
          const assetPath = "/api/workspaces/" + workspace + "/flat-question-assets";
          if (url.pathname === assetPath && method === "GET") {
            return json([hotspotAsset], 200, { "cache-control": "no-store" });
          }
          if (url.pathname === assetPath && method === "POST") {
            return json(hotspotAsset, 201, { "cache-control": "no-store" });
          }
          if (url.pathname === sourcePath && method === "GET") {
            return new Response(serializeFlatQuestionSource(source), {
              headers: { "content-type": "application/vnd.peptidyle.flat-question+json", etag: '"' + revision + '"' },
            });
          }
          if (url.pathname === sourcePath && method === "PUT") {
            if (staleNextSave) {
              staleNextSave = false;
              source = { ...source, title: "Remote collaborator title" };
              revision += 1;
              return json({ error: "stale" }, 409);
            }
            const candidate = JSON.parse(body);
            const finish = () => {
              source = candidate;
              revision += 1;
              return json(compiled(source), 200, { etag: '"' + revision + '"' });
            };
            if (delaySave) return await new Promise((resolve) => { releaseSave = () => resolve(finish()); });
            return finish();
          }
          if (url.pathname === "/api/problems/" + workspace + "/flat-question-publish" && method === "POST") {
            return json(compiled(source, true));
          }
          return json({ error: "not found" }, 404);
        };
        const client = createFlatQuestionClient({ fetch: transport });
        const assetClient = createFlatQuestionAssetClient({ fetch: transport });
        const repository = createFlatQuestionRepository(client);
        const api = {
          validateWorkspacePublication: async () => {
            publicCalls.push({ kind: "validate", body: null });
            if (delayValidation) await new Promise((resolve) => { releaseValidation = resolve; });
            return { kind: "capabilityReport", revision: '"' + revision + '"', violations: [] };
          },
          getWorkspacePublicationDiff: async () => {
            publicCalls.push({ kind: "diff", body: JSON.stringify({ title: source.title, changed: ["title"] }) });
            return {
              draftRevision: revision, revision: '"' + revision + '"', baseline: "firstPublication", prior: null,
              previous: null, changed: ["title"], current: {
                sourceBackend: "native", title: source.title, prompt: { blocks: ["text"] },
                response: { kind: "multipleChoice", optionCount: 2 },
                attemptPolicy: source.attemptPolicy, timingPolicy: source.timingPolicy,
                randomization: { kind: "static" }, metadata: { tags: source.tags, taxonomy: source.taxonomy,
                  license: source.license, language: source.language },
              },
            };
          },
        };
        const responseValidator = {
          validateResponseFormat: async () => ({ violations: [] }),
        };
        async function mount() {
          dispose?.();
          document.getElementById("flat-fixture")?.remove();
          const host = document.createElement("div");
          host.id = "flat-fixture";
          document.body.appendChild(host);
          const initial = await repository.load(workspace);
          dispose = render(() => createComponent(FlatQuestionEditorPage, { workspace, initial, repository, api, responseValidator, assetClient }), host);
        }
        window.__flatQuestionFixture = {
          calls, publicCalls, mount,
          delayNextSave() { delaySave = true; },
          releaseSave() { delaySave = false; releaseSave?.(); releaseSave = null; },
          staleNextSave() { staleNextSave = true; },
          delayValidation() { delayValidation = true; },
          releaseValidation() { delayValidation = false; releaseValidation?.(); releaseValidation = null; },
          source: () => source,
        };
        void mount();
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "flat_question_editor_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles[0];
  if (output === undefined)
    throw new Error("Flat-question editor fixture bundle was not produced.");
  fixtureScript = output.text;
});

async function fixture(page: import("@playwright/test").Page): Promise<void> {
  await page.goto("/");
  await page.addScriptTag({ content: fixtureScript });
  await expect(page.getByRole("heading", { name: "Flat question" })).toBeVisible();
}

async function fixtureValue<T>(
  page: import("@playwright/test").Page,
  expression: string,
): Promise<T> {
  return await page.evaluate<T>(expression);
}

test("instructor authors, resolves a stale draft, and publishes only the reviewed revision", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  page.on("pageerror", (error) => consoleErrors.push(error.message));
  await fixture(page);

  await expect(page.getByLabel("Question title")).toHaveValue("Favorite color");
  await page.getByLabel("Question title").fill("Favorite color revised");
  await page.getByLabel("Learner-facing prompt").fill("Which color do I prefer today?");
  await page.getByRole("button", { name: "Add choice" }).click();
  const choiceText = page.getByLabel("Choice text");
  await choiceText.nth(0).fill("Blue");
  await choiceText.nth(1).fill("Red");
  await choiceText.nth(2).fill("Yellow");
  await page.getByRole("radio", { name: /Mark choice 1 as correct:/u }).check();
  await page
    .getByLabel("Teaching feedback for this choice (optional)")
    .nth(0)
    .fill("Blue is correct.");
  await page
    .getByLabel("Correct-answer feedback (optional)", { exact: true })
    .fill("Exactly right.");
  await page.getByLabel("Incorrect-answer feedback (optional)", { exact: true }).fill("Try again.");

  const beforePreview = await fixtureValue<number>(
    page,
    "window.__flatQuestionFixture.calls.length",
  );
  const preview = page.getByRole("region", { name: "Student preview" });
  await preview.getByLabel(/Blue/).check();
  await expect(preview).not.toContainText("Exactly right.");
  await expect(preview).not.toContainText("correctChoice");
  expect(await fixtureValue<number>(page, "window.__flatQuestionFixture.calls.length")).toBe(
    beforePreview,
  );
  expect((await preview.innerText()).toLowerCase()).not.toMatch(
    /feedback|correctchoice|checksum|base64|private/,
  );

  await page.evaluate("window.__flatQuestionFixture.delayNextSave()");
  const save = page.getByRole("button", { name: "Save private draft" });
  await save.click();
  await save.click({ force: true });
  await expect(save).toBeDisabled();
  expect(
    await fixtureValue<number>(
      page,
      "window.__flatQuestionFixture.calls.filter((call) => call.method === 'PUT').length",
    ),
  ).toBe(1);
  await page.evaluate("window.__flatQuestionFixture.releaseSave()");
  await expect(page.getByRole("button", { name: "Check instructor answer" })).toBeEnabled();
  expect(
    await fixtureValue<string>(
      page,
      "window.__flatQuestionFixture.source().response.choices[2].text",
    ),
  ).toBe("Yellow");

  await page.evaluate("window.__flatQuestionFixture.mount()");
  await expect(page.getByLabel("Question title")).toHaveValue("Favorite color revised");
  await expect(page.getByLabel("Learner-facing prompt")).toHaveValue(
    "Which color do I prefer today?",
  );
  await expect(page.getByLabel("Teaching feedback for this choice (optional)").nth(0)).toHaveValue(
    "Blue is correct.",
  );
  await expect(page.getByLabel("Correct-answer feedback (optional)", { exact: true })).toHaveValue(
    "Exactly right.",
  );
  await expect(
    page.getByLabel("Incorrect-answer feedback (optional)", { exact: true }),
  ).toHaveValue("Try again.");

  await page.getByLabel("Question title").fill("My local conflict edit");
  await page.evaluate("window.__flatQuestionFixture.staleNextSave()");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(page.getByRole("alert")).toContainText("newer saved draft exists");
  await expect(page.getByLabel("Question title")).toHaveValue("My local conflict edit");
  await page.getByRole("button", { name: "Reload newest draft" }).click();
  await expect(page.getByLabel("Question title")).toHaveValue("Remote collaborator title");

  await page.getByLabel("Question title").fill("Ready after review race");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(page.getByRole("button", { name: "Check instructor answer" })).toBeEnabled();
  await page.evaluate("window.__flatQuestionFixture.delayValidation()");
  await page.getByRole("button", { name: "Review publication changes" }).click();
  await page.getByLabel("Question title").fill("Changed during review");
  await page.evaluate("window.__flatQuestionFixture.releaseValidation()");
  await expect(page.getByRole("button", { name: "Confirm and publish" })).toHaveCount(0);
  expect(
    await fixtureValue<number>(
      page,
      "window.__flatQuestionFixture.calls.filter((call) => call.method === 'POST').length",
    ),
  ).toBe(0);

  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(page.getByRole("button", { name: "Check instructor answer" })).toBeEnabled();
  await page.getByRole("button", { name: "Review publication changes" }).click();
  await expect(page.getByRole("button", { name: "Confirm and publish" })).toBeVisible();
  await page.getByLabel("Publication scope").selectOption("public");
  await page.getByRole("button", { name: "Confirm and publish" }).click();
  await expect(page.getByRole("link", { name: "Open question library" })).toHaveAttribute(
    "href",
    "/library",
  );

  const protectedPut = await fixtureValue<{ readonly body: string; readonly ifMatch: string }[]>(
    page,
    "window.__flatQuestionFixture.calls.filter((call) => call.method === 'PUT')",
  );
  const lastProtectedPut = protectedPut[protectedPut.length - 1];
  if (lastProtectedPut === undefined) throw new Error("Expected a protected source save.");
  expect(lastProtectedPut.ifMatch).toMatch(/^"[1-9][0-9]*"$/u);
  expect(lastProtectedPut.body).toContain('"correctChoice"');
  const publish = await fixtureValue<{ readonly body: string; readonly ifMatch: string }[]>(
    page,
    "window.__flatQuestionFixture.calls.filter((call) => call.method === 'POST')",
  );
  expect(publish).toHaveLength(1);
  expect(publish[0]).toMatchObject({
    body: '{"scope":"public"}',
    ifMatch: expect.stringMatching(/^"[1-9][0-9]*"$/u),
  });
  const publicBodies = await fixtureValue<{ readonly body: string | null }[]>(
    page,
    "window.__flatQuestionFixture.publicCalls",
  );
  for (const call of [...publicBodies, ...publish]) {
    expect((call.body ?? "").toLowerCase()).not.toMatch(
      /correctchoice|private|checksum|base64|key|answer/,
    );
  }
  expect(consoleErrors).toEqual([]);
});

test("native controls remain keyboard reachable and narrow viewports do not overflow", async ({
  page,
}) => {
  await page.setViewportSize({ width: 375, height: 800 });
  await fixture(page);
  await page.getByLabel("Question title").focus();
  await page.keyboard.press("Meta+A");
  await page.keyboard.type("Keyboard title");
  await expect(page.getByLabel("Question title")).toHaveValue("Keyboard title");
  const secondCorrectChoice = page.getByRole("radio", { name: /Mark choice 2 as correct:/u });
  await secondCorrectChoice.focus();
  await page.keyboard.press("Space");
  await expect(secondCorrectChoice).toBeChecked();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
});

test("instructor authors matching pairs while student preview remains answer-free", async ({
  page,
}) => {
  await fixture(page);
  await page.getByLabel("Question format").selectOption("matching");
  const firstPrompt = page.getByRole("textbox", { name: "Prompt 1" });
  await firstPrompt.fill("");
  await firstPrompt.pressSequentially("Gene");
  await expect(firstPrompt).toHaveValue("Gene");
  await page.getByLabel("Choice 1").fill("DNA segment");
  await page.getByLabel(/Pair prompt 1:/u).selectOption("choice_a");
  await page.getByLabel(/Pair prompt 2:/u).selectOption("choice_b");
  const preview = page.getByRole("region", { name: "Student preview" });
  await expect(preview.getByRole("group", { name: /Gene/u })).toBeVisible();
  await expect(preview).not.toContainText("Private pairing check");
  await expect(preview).not.toContainText("matches");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(page.getByRole("button", { name: "Check instructor answer" })).toBeEnabled();
  const saved = await fixtureValue<string>(
    page,
    "window.__flatQuestionFixture.source().response.kind",
  );
  expect(saved).toBe("matching");
  expect(
    await fixtureValue<string>(
      page,
      "JSON.stringify(window.__flatQuestionFixture.source().response.matches)",
    ),
  ).toBe('[{"prompt":"prompt_a","choice":"choice_a"},{"prompt":"prompt_b","choice":"choice_b"}]');
});

test("matching author can add a third pair, reorder it, and save with keyboard-visible controls", async ({
  page,
}) => {
  await fixture(page);
  await page.getByLabel("Question format").selectOption("matching");
  await page.getByRole("button", { name: "Add pair" }).click();
  const thirdPrompt = page.getByRole("textbox", { name: "Prompt 3" });
  await expect(thirdPrompt).toBeVisible();
  await thirdPrompt.focus();
  await page.keyboard.press("Meta+A");
  await page.keyboard.type("Codon");
  await expect(thirdPrompt).toHaveValue("Codon");
  await page.getByRole("button", { name: "Earlier" }).nth(2).click();
  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(page.getByRole("button", { name: "Check instructor answer" })).toBeEnabled();
  expect(
    await fixtureValue<number>(
      page,
      "window.__flatQuestionFixture.source().response.prompts.length",
    ),
  ).toBe(3);
});

test("an incomplete numeric literal blocks saving and publication review until the literal is complete", async ({
  page,
}) => {
  await fixture(page);
  await page.getByLabel("Question format").selectOption("numeric");
  const numeric = page.getByLabel("Accepted numeric value");
  await numeric.fill("6.02e");
  await expect(page.getByRole("alert")).toContainText("Finish the numeric value");
  await expect(page.getByRole("button", { name: "Save private draft" })).toBeDisabled();
  await expect(page.getByRole("button", { name: "Review publication changes" })).toBeDisabled();
  await numeric.fill("6.02e23");
  await expect(page.getByRole("button", { name: "Save private draft" })).toBeEnabled();
});

test("instructor completes each additional response format with keyboard edits, save, and answer-free preview", async ({
  page,
}) => {
  await fixture(page);
  const format = page.getByLabel("Question format");
  const title = page.getByLabel("Question title");
  const preview = page.getByRole("region", { name: "Student preview" });

  await format.selectOption("multipleAnswer");
  await page.getByLabel("Choice text").nth(0).focus();
  await page.keyboard.press("Meta+A");
  await page.keyboard.type("Kinase");
  await title.fill("Multiple answer task");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(preview.getByRole("checkbox", { name: /Kinase/u })).toBeVisible();

  await format.selectOption("fillIn");
  await page.getByLabel("Accepted answer 1").focus();
  await page.keyboard.press("Meta+A");
  await page.keyboard.type("ATP synthase");
  await title.fill("Fill in task");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(preview.getByRole("textbox")).toBeVisible();
  await expect(preview).not.toContainText("ATP synthase");

  await format.selectOption("multiFillIn");
  await page.getByLabel(/Visible blank label/u).focus();
  await page.keyboard.press("Meta+A");
  await page.keyboard.type("Gene");
  await title.fill("Multiple fill task");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(preview.getByRole("textbox", { name: /Gene/u })).toBeVisible();
  await expect(preview).not.toContainText("Accepted answer");

  await format.selectOption("numeric");
  const numeric = page.getByLabel("Accepted numeric value");
  await numeric.focus();
  await page.keyboard.press("Meta+A");
  await page.keyboard.type("6.02e");
  await expect(numeric).toHaveValue("6.02e");
  await page.keyboard.type("23");
  await expect(numeric).toHaveValue("6.02e23");
  await title.fill("Numeric task");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(preview.getByRole("spinbutton")).toBeVisible();
  await expect(preview).not.toContainText("6.02e23");

  await format.selectOption("ordering");
  await page.getByLabel("Item text").nth(0).focus();
  await page.keyboard.press("Meta+A");
  await page.keyboard.type("Transcription");
  await title.fill("Ordering task");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(preview.getByText("Transcription", { exact: true })).toBeVisible();
  await expect(preview).not.toContainText("correctOrder");
});

test("instructor format chooser is accessible at the canonical laptop target", async ({ page }) => {
  await page.setViewportSize({ width: 1_280, height: 800 });
  await fixture(page);
  await expect(page.getByLabel("Question format")).toBeVisible();
  await page.getByLabel("Question format").selectOption("multipleAnswer");
  const results = await new AxeBuilder({ page }).analyze();
  const blocking = results.violations.filter(
    (violation) => violation.impact === "critical" || violation.impact === "serious",
  );
  expect(blocking).toEqual([]);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
});

test("author creates a keyboard-first hotspot from a verified image without exposing its answer key", async ({
  page,
}) => {
  await fixture(page);
  const format = page.getByLabel("Question format");
  await format.selectOption("hotspot");
  await expect(
    page.getByText(
      "Choose a verified image and describe it before the hotspot draft can be saved.",
    ),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Save private draft" })).toBeDisabled();
  const image = page.getByRole("group", { name: "Image" }).getByRole("combobox", { name: "Image" });
  await image.focus();
  await image.selectOption("aaaaaaaa-0000-4000-8000-000000000013");
  await page
    .getByLabel("Image description for learners")
    .fill("A cell membrane diagram with labeled transport features.");
  await expect(page.getByLabel("Region label").nth(0)).toBeVisible();
  await page.getByLabel("Region label").nth(0).fill("Protein channel");
  await page.getByRole("button", { name: "Add labeled region" }).click();
  await page.getByLabel("Region label").nth(1).fill("Phospholipid tail");
  await page.getByRole("button", { name: "Later" }).nth(0).click();
  await expect(page.getByRole("heading", { name: "Region 1" })).toBeVisible();
  await page.getByRole("checkbox", { name: "Correct region" }).nth(0).check();
  await page.getByRole("button", { name: "Remove region" }).nth(1).click();
  await expect(page.getByLabel("Region label")).toHaveCount(1);
  const preview = page.getByRole("region", { name: "Student preview" });
  await expect(preview.getByRole("checkbox", { name: "Phospholipid tail" })).toBeVisible();
  await expect(preview).not.toContainText("correctRegions");
  await expect(preview).not.toContainText("aaaaaaaa-0000-4000-8000-000000000013");
  await page.getByRole("button", { name: "Save private draft" }).click();
  await expect(page.getByRole("button", { name: "Check instructor answer" })).toBeEnabled();
  await page.setViewportSize({ width: 1_280, height: 800 });
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
  const results = await new AxeBuilder({ page }).analyze();
  expect(
    results.violations.filter((item) => item.impact === "critical" || item.impact === "serious"),
  ).toEqual([]);
});
