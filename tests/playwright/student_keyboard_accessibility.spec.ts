// student_keyboard_accessibility.spec.ts - keyboard-only response-family acceptance.

import { expect, test, type Locator, type Page } from "@playwright/test";
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
        import { ResponseWidget } from "./src/components/response_widget.tsx";

        const validator = {
          mode: "wasm",
          validateResponseFormat: async () => ({ violations: [] }),
        };
        const text = (id, markdown) => ({ id, body: [{ kind: "text", markdown }] });

        const platformRoot = document.createElement("section");
        platformRoot.id = "platform-keyboard-fixture";
        platformRoot.innerHTML = "<h2>Platform keyboard fixture</h2>";
        document.body.append(platformRoot);
        render(() => createComponent(ResponseWidget, {
          attemptId: "platform-keyboard-attempt",
          definition: {
            kind: "multipleChoice",
            choices: [text("alpha", "Alpha"), text("beta", "Beta"), text("gamma", "Gamma")],
            selection: { kind: "exactlyOne" },
          },
          validator,
          onSubmit: async () => { platformRoot.dataset.submitted = "true"; },
          onEscape: () => { platformRoot.dataset.escaped = "true"; },
        }), platformRoot);

        const orderingRoot = document.createElement("section");
        orderingRoot.id = "ordering-keyboard-fixture";
        orderingRoot.innerHTML = "<h2>Ordering keyboard fixture</h2>";
        document.body.append(orderingRoot);
        render(() => createComponent(ResponseWidget, {
          attemptId: "ordering-keyboard-attempt",
          definition: {
            kind: "ordering",
            items: [text("first", "First"), text("second", "Second"), text("third", "Third")],
          },
          validator,
          onSubmit: async () => { orderingRoot.dataset.submitted = "true"; },
          onEscape: () => { orderingRoot.dataset.escaped = "true"; },
        }), orderingRoot);

        const multipleRoot = document.createElement("section");
        multipleRoot.id = "multiple-keyboard-fixture";
        multipleRoot.innerHTML = "<h2>Multiple-answer keyboard fixture</h2>";
        document.body.append(multipleRoot);
        render(() => createComponent(ResponseWidget, {
          attemptId: "multiple-keyboard-attempt",
          definition: {
            kind: "multipleChoice",
            choices: [text("alpha", "Alpha"), text("beta", "Beta"), text("gamma", "Gamma")],
            selection: { kind: "anyNumber" },
          },
          validator,
          onSubmit: async () => { multipleRoot.dataset.submitted = "true"; },
          onEscape: () => { multipleRoot.dataset.escaped = "true"; },
        }), multipleRoot);

        const blankRoot = document.createElement("section");
        blankRoot.id = "multi-blank-keyboard-fixture";
        blankRoot.innerHTML = "<h2>Multi-blank keyboard fixture</h2>";
        document.body.append(blankRoot);
        render(() => createComponent(ResponseWidget, {
          attemptId: "multi-blank-keyboard-attempt",
          definition: {
            kind: "multiBlank",
            blanks: [
              { id: "first", label: [{ kind: "text", markdown: "First blank" }], matchMode: "normalized", maxLength: 20 },
              { id: "second", label: [{ kind: "text", markdown: "Second blank" }], matchMode: "normalized", maxLength: 20 },
            ],
          },
          validator,
          onSubmit: async () => { blankRoot.dataset.submitted = "true"; },
          onEscape: () => { blankRoot.dataset.escaped = "true"; },
        }), blankRoot);

        const matchingRoot = document.createElement("section");
        matchingRoot.id = "matching-keyboard-fixture";
        matchingRoot.innerHTML = "<h2>Matching keyboard fixture</h2>";
        document.body.append(matchingRoot);
        render(() => createComponent(ResponseWidget, {
          attemptId: "matching-keyboard-attempt",
          definition: {
            kind: "matching",
            prompts: [text("dna", "DNA"), text("rna", "RNA")],
            choices: [
              text("deoxy", "Deoxyribose"),
              text("ribose", "Ribose"),
              text("phosphate", "Phosphate"),
            ],
          },
          validator,
          onSubmit: async (response) => {
            matchingRoot.dataset.submitted = "true";
            matchingRoot.dataset.response = JSON.stringify(response);
          },
          onEscape: () => { matchingRoot.dataset.escaped = "true"; },
        }), matchingRoot);

        const hotspotRoot = document.createElement("section");
        hotspotRoot.id = "hotspot-keyboard-fixture";
        hotspotRoot.innerHTML = "<h2>Hotspot keyboard fixture</h2>";
        document.body.append(hotspotRoot);
        render(() => createComponent(ResponseWidget, {
          attemptId: "hotspot-keyboard-attempt",
          definition: {
            kind: "hotspot",
            surface: { asset: "00000000-0000-0000-0000-000000000123", checksum: "1111111111111111111111111111111111111111111111111111111111111111" },
            description: "Cell diagram",
            regions: [
              { id: "nucleus", label: [{ kind: "text", markdown: "Nucleus" }], x: 1000, y: 1000, width: 2000, height: 2000 },
              { id: "membrane", label: [{ kind: "text", markdown: "Cell membrane" }], x: 6000, y: 6000, width: 2000, height: 2000 },
            ],
            selection: { kind: "exactlyOne" },
          },
          validator,
          onSubmit: async () => { hotspotRoot.dataset.submitted = "true"; },
          onEscape: () => { hotspotRoot.dataset.escaped = "true"; },
        }), hotspotRoot);
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "student_keyboard_accessibility_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles[0];
  if (output === undefined) throw new Error("Student keyboard fixture bundle was not produced.");
  fixtureScript = output.text;
});

async function mountFixture(page: Page): Promise<void> {
  await page.goto("/");
  await page.addScriptTag({ content: fixtureScript });
}

async function tabTo(page: Page, target: Locator, limit = 80): Promise<void> {
  for (let index = 0; index < limit; index += 1) {
    if (await target.evaluate((element) => document.activeElement === element)) return;
    await page.keyboard.press("Tab");
  }
  throw new Error(`Tab did not reach ${await target.getAttribute("aria-label")}`);
}

test("platform contract uses Tab, Shift+Tab, and Space from response through submit", async ({
  page,
}) => {
  await mountFixture(page);
  const fixture = page.locator("#platform-keyboard-fixture");
  const radios = fixture.getByRole("radio");
  const submit = fixture.getByRole("button", { name: "Submit answer" });

  await tabTo(page, radios.first());
  await page.keyboard.press("Space");
  await expect(radios.first()).toBeChecked();
  await expect(fixture.getByRole("status", { name: "Response format" })).toContainText(
    "ready to submit",
  );

  await page.keyboard.press("Tab");
  await expect(submit).toBeFocused();
  await page.keyboard.press("Shift+Tab");
  await expect(radios.first()).toBeFocused();
  await page.keyboard.press("Tab");
  await expect(submit).toBeFocused();
  await page.keyboard.press("Space");
  await expect(fixture).toHaveAttribute("data-submitted", "true");
});

test("platform contract uses Tab and Space for multiple-answer selection and submit", async ({
  page,
}) => {
  await mountFixture(page);
  const fixture = page.locator("#multiple-keyboard-fixture");
  const firstCheckbox = fixture.getByRole("checkbox").first();
  const submit = fixture.getByRole("button", { name: "Submit answer" });

  await tabTo(page, firstCheckbox);
  await page.keyboard.press("Space");
  await expect(firstCheckbox).toBeChecked();
  await tabTo(page, submit);
  await page.keyboard.press("Space");
  await expect(fixture).toHaveAttribute("data-submitted", "true");
});

test("platform contract uses visible ordering buttons with Tab and Space", async ({ page }) => {
  await mountFixture(page);
  const fixture = page.locator("#ordering-keyboard-fixture");
  const firstLater = fixture.getByRole("button", { name: "Move item 1 later" });
  const submit = fixture.getByRole("button", { name: "Submit answer" });

  await tabTo(page, firstLater);
  await page.keyboard.press("Space");
  await expect(fixture.locator(".ordering-row > span")).toHaveText(["Second", "First", "Third"]);
  await tabTo(page, submit);
  await page.keyboard.press("Space");
  await expect(fixture).toHaveAttribute("data-submitted", "true");
});

test("platform contract uses Tab and typing for every multi-blank field", async ({ page }) => {
  await mountFixture(page);
  const fixture = page.locator("#multi-blank-keyboard-fixture");
  const fields = fixture.getByRole("textbox");
  await tabTo(page, fields.first());
  await page.keyboard.type("adenine");
  await page.keyboard.press("Tab");
  await expect(fields.nth(1)).toBeFocused();
  await page.keyboard.type("cytosine");
  await tabTo(page, fixture.getByRole("button", { name: "Submit answer" }));
  await page.keyboard.press("Space");
  await expect(fixture).toHaveAttribute("data-submitted", "true");
});

test("platform contract uses Tab, Shift+Tab, and Space for matching", async ({ page }) => {
  await mountFixture(page);
  const fixture = page.locator("#matching-keyboard-fixture");
  const dna = fixture.getByRole("group", { name: "DNA" });
  const rna = fixture.getByRole("group", { name: "RNA" });
  const dnaDeoxyribose = dna.getByRole("radio", { name: /Deoxyribose/u });
  const rnaRibose = rna.getByRole("radio", { name: /Ribose/u });
  await tabTo(page, dnaDeoxyribose);
  await expect(dnaDeoxyribose).toHaveCSS("outline-style", "solid");
  await page.keyboard.press("Space");
  await expect(dnaDeoxyribose).toHaveAttribute("aria-checked", "true");
  await tabTo(page, rnaRibose);
  await page.keyboard.press("Space");
  await expect(rnaRibose).toHaveAttribute("aria-checked", "true");
  await page.keyboard.press("Shift+Tab");
  const previousAvailablePairing = fixture.locator(
    '[role="radio"]:focus:not([aria-disabled="true"])',
  );
  await expect(previousAvailablePairing).toHaveCount(1);
  await expect(previousAvailablePairing).toHaveAttribute("aria-disabled", "false");
  await tabTo(page, fixture.getByRole("button", { name: "Submit answer" }));
  await page.keyboard.press("Space");
  await expect(fixture).toHaveAttribute("data-submitted", "true");
});

test("native matching retains arrow navigation as an optional radio-group extension", async ({
  page,
}) => {
  await mountFixture(page);
  const fixture = page.locator("#matching-keyboard-fixture");
  const dna = fixture.getByRole("group", { name: "DNA" });
  const dnaDeoxyribose = dna.getByRole("radio", { name: /Deoxyribose/u });
  const dnaRibose = dna.getByRole("radio", { name: /Ribose/u });

  await tabTo(page, dnaDeoxyribose);
  await page.keyboard.press("Space");
  await page.keyboard.press("ArrowDown");
  await expect(dnaRibose).toBeFocused();
  await expect(dnaRibose).toHaveAttribute("aria-checked", "true");
});

test("native matching visibly tracks exclusive keyboard selections without exposing an answer key", async ({
  page,
}) => {
  await mountFixture(page);
  const fixture = page.locator("#matching-keyboard-fixture");
  const progress = fixture.getByRole("status").filter({ hasText: "0 of 2 prompts matched" });
  const dna = fixture.getByRole("group", { name: "DNA" });
  const rna = fixture.getByRole("group", { name: "RNA" });
  const dnaDeoxyribose = dna.getByRole("radio", { name: /Deoxyribose/u });
  const dnaRibose = dna.getByRole("radio", { name: /Ribose/u });
  const rnaDeoxyribose = rna.getByRole("radio", { name: /Deoxyribose/u });

  await expect(progress).toBeVisible();
  await tabTo(page, dnaDeoxyribose);
  await page.keyboard.press("Space");
  await expect(dnaDeoxyribose).toHaveAttribute("aria-checked", "true");
  await expect(dnaDeoxyribose).toHaveAttribute("aria-disabled", "false");
  await expect(dna.getByText("Selected for this prompt.", { exact: true })).toBeVisible();
  await expect(rnaDeoxyribose).toHaveAttribute("aria-disabled", "true");
  await expect(rnaDeoxyribose).toHaveAccessibleName(
    /Deoxyribose.*Already selected for another prompt/iu,
  );
  await expect(
    rna.getByText("Already selected for another prompt.", { exact: true }),
  ).toBeVisible();
  await expect(
    fixture.getByRole("status").filter({ hasText: "1 of 2 prompts matched" }),
  ).toBeVisible();

  await page.keyboard.press("ArrowDown");
  await expect(dnaRibose).toBeFocused();
  await expect(dnaRibose).toHaveAttribute("aria-checked", "true");
  await expect(dna.getByText("Selected for this prompt.", { exact: true })).toBeVisible();
  await expect(rnaDeoxyribose).toHaveAttribute("aria-disabled", "false");
  await expect(rnaDeoxyribose).toHaveAccessibleName(/Deoxyribose.*Available/iu);

  await tabTo(page, rnaDeoxyribose);
  await expect(rnaDeoxyribose).toBeFocused();
  await page.keyboard.press("Space");
  await expect(rnaDeoxyribose).toHaveAttribute("aria-checked", "true");
  await expect(
    fixture.getByRole("status").filter({ hasText: "2 of 2 prompts matched" }),
  ).toBeVisible();

  await tabTo(page, fixture.getByRole("button", { name: "Submit answer" }));
  await page.keyboard.press("Space");
  await expect(fixture).toHaveAttribute("data-submitted", "true");
  const submitted = await fixture.getAttribute("data-response");
  expect(submitted).not.toBeNull();
  expect(submitted).not.toMatch(/answer|correct|key/iu);
  expect(JSON.parse(submitted ?? "")).toEqual({
    kind: "matching",
    matches: [
      { prompt: "dna", choice: "ribose" },
      { prompt: "rna", choice: "deoxy" },
    ],
  });
});

test("matching remains keyboard-visible and horizontally usable across learner viewports", async ({
  page,
}) => {
  for (const width of [320, 480, 768, 1_920]) {
    await page.setViewportSize({ width, height: 900 });
    await mountFixture(page);

    const fixture = page.locator("#matching-keyboard-fixture");
    const dna = fixture.getByRole("group", { name: "DNA" });
    const firstPairing = dna.getByRole("radio", { name: /Deoxyribose/u });

    await tabTo(page, firstPairing);
    await expect(firstPairing).toBeFocused();
    await expect(firstPairing).toBeInViewport();
    await expect(firstPairing).toHaveCSS("outline-style", "solid");
    await page.keyboard.press("Space");
    await expect(firstPairing).toHaveAttribute("aria-checked", "true");
    expect(
      await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
    ).toBe(true);
  }
});

test("platform contract makes the hotspot region list fully no-mouse operable", async ({
  page,
}) => {
  await mountFixture(page);
  const fixture = page.locator("#hotspot-keyboard-fixture");
  const regions = fixture.getByRole("radio");
  await tabTo(page, regions.first());
  await page.keyboard.press("Space");
  await expect(regions.first()).toBeChecked();
  await page.keyboard.press("Tab");
  const submit = fixture.getByRole("button", { name: "Submit answer" });
  await expect(submit).toBeFocused();
  await page.keyboard.press("Space");
  await expect(fixture).toHaveAttribute("data-submitted", "true");
});

test("extension: ordering arrows move the item and announce its new position", async ({ page }) => {
  await mountFixture(page);
  const fixture = page.locator("#ordering-keyboard-fixture");
  const firstLater = fixture.getByRole("button", { name: "Move item 1 later" });

  await tabTo(page, firstLater);
  await page.keyboard.press("ArrowDown");
  await expect(fixture.locator(".ordering-row > span")).toHaveText(["Second", "First", "Third"]);
  await expect(fixture.getByRole("button", { name: "Move item 2 later" })).toBeFocused();
  await expect(fixture.getByRole("status").first()).toContainText("First moved to position 2");

  await page.keyboard.press("ArrowDown");
  await expect(fixture.locator(".ordering-row > span")).toHaveText(["Second", "Third", "First"]);
  await expect(fixture.getByRole("button", { name: "Move item 3 earlier" })).toBeFocused();
});

test("extension: multiple-answer arrows move focus without changing selection", async ({
  page,
}) => {
  await mountFixture(page);
  const fixture = page.locator("#multiple-keyboard-fixture");
  const checkboxes = fixture.getByRole("checkbox");

  await tabTo(page, checkboxes.first());
  await page.keyboard.press("ArrowRight");
  await expect(checkboxes.nth(1)).toBeFocused();
  await expect(checkboxes.nth(1)).not.toBeChecked();
  await page.keyboard.press("Space");
  await expect(checkboxes.nth(1)).toBeChecked();
  await expect(fixture.getByRole("status", { name: "Response format" })).toContainText(
    "ready to submit",
  );
});

test("extension: radio arrows use the native group selection convention", async ({ page }) => {
  await mountFixture(page);
  const fixture = page.locator("#platform-keyboard-fixture");
  const radios = fixture.getByRole("radio");

  await tabTo(page, radios.first());
  await page.keyboard.press("ArrowDown");
  await expect(radios.nth(1)).toBeFocused();
  await expect(radios.nth(1)).toBeChecked();
});

test("extension: a digit selects a visible choice only from a choice input", async ({ page }) => {
  await mountFixture(page);
  const fixture = page.locator("#platform-keyboard-fixture");
  const radios = fixture.getByRole("radio");
  const submit = fixture.getByRole("button", { name: "Submit answer" });

  await tabTo(page, radios.first());
  await page.keyboard.press("2");
  await expect(radios.nth(1)).toBeFocused();
  await expect(radios.nth(1)).toBeChecked();

  await page.keyboard.press("Tab");
  await expect(submit).toBeFocused();
  await page.keyboard.press("3");
  await expect(radios.nth(1)).toBeChecked();
  await expect(radios.nth(2)).not.toBeChecked();
});

test("extension: Enter submits a ready response from its response input", async ({ page }) => {
  await mountFixture(page);
  const fixture = page.locator("#platform-keyboard-fixture");
  const firstRadio = fixture.getByRole("radio").first();

  await tabTo(page, firstRadio);
  await page.keyboard.press("Space");
  await expect(firstRadio).toBeChecked();
  await page.keyboard.press("Enter");
  await expect(fixture).toHaveAttribute("data-submitted", "true");
});

test("extension: Escape invokes the response widget's safe return action", async ({ page }) => {
  await mountFixture(page);
  const fixture = page.locator("#platform-keyboard-fixture");

  await tabTo(page, fixture.getByRole("radio").first());
  await page.keyboard.press("Escape");
  await expect(fixture).toHaveAttribute("data-escaped", "true");
});
