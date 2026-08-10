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
          onEscape: () => {},
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
          onEscape: () => {},
        }), multipleRoot);
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

async function tabTo(page: Page, target: Locator, limit = 30): Promise<void> {
  for (let index = 0; index < limit; index += 1) {
    if (await target.evaluate((element) => document.activeElement === element)) return;
    await page.keyboard.press("Tab");
  }
  throw new Error(`Tab did not reach ${await target.getAttribute("aria-label")}`);
}

test("ordering supports Tab, arrow movement, announcements, and Enter submit", async ({ page }) => {
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

  const submit = fixture.getByRole("button", { name: "Submit answer" });
  await tabTo(page, submit);
  await page.keyboard.press("Enter");
  await expect(fixture).toHaveAttribute("data-submitted", "true");
});

test("multiple-answer checkboxes support arrow focus, Space selection, and Enter submit", async ({
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

  await page.keyboard.press("Enter");
  await expect(fixture).toHaveAttribute("data-submitted", "true");
});
