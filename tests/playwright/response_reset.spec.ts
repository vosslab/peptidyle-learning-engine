// response_reset.spec.ts - learner-visible reset contract for native response families.

import { expect, test, type Page } from "@playwright/test";
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

        const validator = { mode: "wasm", validateResponseFormat: async () => ({ violations: [] }) };
        const text = (id, markdown) => ({ id, body: [{ kind: "text", markdown }] });
        const fixtures = [
          ["mc", { kind: "multipleChoice", choices: [text("a", "Alpha"), text("b", "Beta")], selection: { kind: "exactlyOne" } }, { kind: "multipleChoice", selected: ["b"] }],
          ["ma", { kind: "multipleChoice", choices: [text("a", "Alpha"), text("b", "Beta"), text("c", "Gamma")], selection: { kind: "atLeastOne" } }, { kind: "multipleChoice", selected: ["b"] }],
          ["num", { kind: "numeric", tolerance: { kind: "exact" }, unit: null }, { kind: "numeric", value: 7 }],
          ["fib", { kind: "shortText", maxLength: 40 }, { kind: "shortText", text: "resume" }],
          ["multi", { kind: "multiBlank", blanks: [{ id: "one", label: [{ kind: "text", markdown: "One" }], matchMode: "normalized", maxLength: 40 }, { id: "two", label: [{ kind: "text", markdown: "Two" }], matchMode: "normalized", maxLength: 40 }] }, { kind: "multiBlank", answers: [{ slot: "one", text: "first" }, { slot: "two", text: "second" }] }],
          ["match", { kind: "matching", prompts: [text("dna", "DNA"), text("rna", "RNA")], choices: [text("d", "Deoxyribose"), text("r", "Ribose")] }, { kind: "matching", matches: [{ prompt: "dna", choice: "d" }, { prompt: "rna", choice: "" }] }],
          ["order", { kind: "ordering", items: [text("one", "One"), text("two", "Two"), text("three", "Three")] }, { kind: "ordering", order: ["three", "one", "two"] }],
          ["hotspot", { kind: "hotspot", description: "Select the highlighted molecular features", regions: [{ id: "helix", label: [{ kind: "text", markdown: "Alpha helix" }], x: 10, y: 10, width: 20, height: 20 }, { id: "sheet", label: [{ kind: "text", markdown: "Beta sheet" }], x: 50, y: 20, width: 20, height: 20 }, { id: "loop", label: [{ kind: "text", markdown: "Surface loop" }], x: 90, y: 30, width: 20, height: 20 }], selection: { kind: "atLeastOne" } }, { kind: "hotspot", points: [{ x: 20, y: 20 }, { x: 60, y: 30 }] }],
        ];
        for (const [id, definition, initialResponse] of fixtures) {
          const root = document.createElement("section");
          root.id = "reset-" + id;
          document.body.append(root);
          render(() => createComponent(ResponseWidget, {
            attemptId: "reset-" + id,
            definition,
            initialResponse,
            validator,
            onSubmit: async () => { root.dataset.submitted = "true"; },
            onEscape: () => undefined,
          }), root);
        }
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "response_reset_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles[0];
  if (output === undefined) throw new Error("Response reset fixture bundle was not produced.");
  fixtureScript = output.text;
});

async function mountFixture(page: Page): Promise<void> {
  await page.goto("/");
  await page.addScriptTag({ content: fixtureScript });
}

async function expectReset(page: Page, id: string, label = "Clear response"): Promise<void> {
  const root = page.locator(`#reset-${id}`);
  const reset = root.getByRole("button", { name: label });
  await reset.click();
  await expect(root.getByRole("status", { name: "Response format" })).toContainText(
    "Response restored",
  );
  await expect(root).not.toHaveAttribute("data-submitted", "true");
}

test("clear response restores exact resumed and issued empty responses without submitting", async ({
  page,
}) => {
  await mountFixture(page);

  const mc = page.locator("#reset-mc");
  await mc.getByRole("radio", { name: /Alpha/ }).check();
  await expectReset(page, "mc");
  await expect(mc.getByRole("radio", { name: /Beta/ })).toBeChecked();
  await expect(mc.getByRole("radio", { name: /Alpha/ })).toBeFocused();

  const ma = page.locator("#reset-ma");
  const selectionCount = ma.getByRole("status", { name: "Selection count" });
  await expect(selectionCount).toHaveText("1 selected. Select at least 1.");
  await ma.getByRole("checkbox", { name: /Alpha/ }).check();
  await expect(selectionCount).toHaveText("2 selected. Select at least 1.");
  await expectReset(page, "ma");
  await expect(ma.getByRole("checkbox", { name: /Alpha/ })).not.toBeChecked();
  await expect(ma.getByRole("checkbox", { name: /Beta/ })).toBeChecked();
  await expect(selectionCount).toHaveText("1 selected. Select at least 1.");
  await expect(ma.getByRole("checkbox", { name: /Alpha/ })).toBeFocused();

  const numeric = page.locator("#reset-num");
  await numeric.getByRole("spinbutton").fill("9");
  await expectReset(page, "num");
  await expect(numeric.getByRole("spinbutton")).toHaveValue("7");
  await expect(numeric.getByRole("spinbutton")).toBeFocused();

  const text = page.locator("#reset-fib");
  await text.getByRole("textbox").fill("changed");
  await expectReset(page, "fib");
  await expect(text.getByRole("textbox")).toHaveValue("resume");
  await expect(text.getByRole("textbox")).toBeFocused();
});

test("clear response restores multi-part matches and Reset order restores order", async ({
  page,
}) => {
  await mountFixture(page);

  const multi = page.locator("#reset-multi");
  const blankCompletion = multi.getByRole("status", { name: "Blank completion" });
  await expect(blankCompletion).toHaveText("2 of 2 blanks completed.");
  await multi.getByRole("textbox").first().fill("");
  await expect(blankCompletion).toHaveText("1 of 2 blanks completed.");
  await expectReset(page, "multi");
  await expect(multi.getByRole("textbox").first()).toHaveValue("first");
  await expect(multi.getByRole("textbox").nth(1)).toHaveValue("second");
  await expect(blankCompletion).toHaveText("2 of 2 blanks completed.");
  await expect(multi.getByRole("textbox").first()).toBeFocused();

  const match = page.locator("#reset-match");
  await match
    .getByRole("radio", { name: /Ribose/ })
    .first()
    .click();
  await expectReset(page, "match");
  await expect(match.getByRole("radio", { name: /Deoxyribose/ }).first()).toHaveAttribute(
    "aria-checked",
    "true",
  );
  await expect(match.getByRole("radio").first()).toBeFocused();

  const order = page.locator("#reset-order");
  await order.getByRole("button", { name: "Move item 1 later" }).click();
  await expectReset(page, "order", "Reset order");
  await expect(order.locator(".ordering-row > span")).toHaveText(["Three", "One", "Two"]);
  await expect(order.getByRole("button", { name: "Move item 1 later" })).toBeFocused();

  const hotspot = page.locator("#reset-hotspot");
  const hotspotSelectionCount = hotspot.getByRole("status", { name: "Selection count" });
  await expect(hotspotSelectionCount).toHaveText("2 selected. Select at least 1.");
  await hotspot.getByRole("checkbox", { name: /Alpha helix/ }).uncheck();
  await hotspot.getByRole("checkbox", { name: /Surface loop/ }).check();
  await expect(hotspotSelectionCount).toHaveText("2 selected. Select at least 1.");
  await expectReset(page, "hotspot");
  await expect(hotspot.getByRole("checkbox", { name: /Alpha helix/ })).toBeChecked();
  await expect(hotspot.getByRole("checkbox", { name: /Beta sheet/ })).toBeChecked();
  await expect(hotspot.getByRole("checkbox", { name: /Surface loop/ })).not.toBeChecked();
  await expect(hotspotSelectionCount).toHaveText("2 selected. Select at least 1.");
  await expect(hotspot.getByRole("checkbox", { name: /Alpha helix/ })).toBeFocused();
});
