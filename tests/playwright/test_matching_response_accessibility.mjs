// MATCH assignment uses accessible choice/slot buttons from
// src/components/question_response_controls/matching.tsx and saves public response pairs.

import assert from "node:assert/strict";
import test from "node:test";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { chromium } from "playwright";

import { REPO_ROOT } from "./repo_root.mjs";

const bundle = await build({
  bundle: true,
  format: "iife",
  globalName: "MatchingFixture",
  outfile: "matching_response.js",
  platform: "browser",
  write: false,
  plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
  stdin: {
    contents: `
      import { createComponent } from "solid-js";
      import { render } from "solid-js/web";
      import { QuestionResponseControl } from "./src/components/question_response_controls/question_response_control.tsx";

      export function mount(target) {
        window.responseSaves = [];
        render(() => createComponent(QuestionResponseControl, {
          attemptId: "attempt-matching",
          mode: "save",
          responseFormat: {
            kind: "matching",
            reuseChoices: false,
            prompts: [
              { id: "prompt-mito", body: [{ kind: "text", markdown: "Mitochondria" }] },
              { id: "prompt-ribo", body: [{ kind: "text", markdown: "Ribosome" }] },
            ],
            choices: [
              { id: "choice-energy", body: [{ kind: "text", markdown: "Makes energy" }] },
              { id: "choice-protein", body: [{ kind: "text", markdown: "Builds proteins" }] },
            ],
          },
          saveLabel: "Save response",
          validator: { validateResponseFormat: async () => ({ issues: [] }) },
          onEscape: () => undefined,
          onSave: async (response) => {
            window.responseSaves.push(response);
            return { kind: "accepted" };
          },
        }), target);
      }
    `,
    resolveDir: REPO_ROOT,
    sourcefile: "matching_response_fixture.tsx",
    loader: "tsx",
  },
});
const javascript = bundle.outputFiles.find((output) => output.path.endsWith(".js"));
assert.ok(javascript, "MATCH fixture must contain JavaScript");

for (const input of ["pointer", "keyboard", "touch"]) {
  test(`MATCH assigns, replaces, clears, and saves using ${input}`, async () => {
    const browser = await chromium.launch({ headless: true });
    try {
      const page = await browser.newPage({
        viewport: { width: input === "touch" ? 360 : 1100, height: 800 },
        hasTouch: input === "touch",
      });
      await page.setContent('<!doctype html><html><body><div id="matching"></div></body></html>');
      await page.addScriptTag({ content: Buffer.from(javascript.contents).toString("utf8") });
      await page.evaluate(() => {
        window.MatchingFixture.mount(document.getElementById("matching"));
      });

      const bank = page.getByRole("group", { name: "Choice bank", exact: true });
      const energy = bank.getByRole("button", { name: /Makes energy/u });
      const protein = bank.getByRole("button", { name: /Builds proteins/u });
      const slot = page.getByRole("button", { name: /^Mitochondria:/u });
      const clear = page.getByRole("button", {
        name: "Clear response for Mitochondria",
        exact: true,
      });
      const save = page.getByRole("button", { name: "Save response", exact: true });

      async function activate(control) {
        if (input === "keyboard") {
          await page.keyboard.press("Tab");
          const first = await page.evaluateHandle(() => document.activeElement);
          try {
            while (!(await control.evaluate((element) => element === document.activeElement))) {
              await page.keyboard.press("Tab");
              assert.equal(
                await page.evaluate((element) => element === document.activeElement, first),
                false,
                "MATCH control must be reachable through keyboard navigation",
              );
            }
            await page.keyboard.press(control === energy ? "Space" : "Enter");
          } finally {
            await first.dispose();
          }
        } else if (input === "touch") {
          await control.tap();
        } else {
          await control.click();
        }
      }

      let saveCount = 0;
      async function assertSaved(choice) {
        await activate(save);
        saveCount += 1;
        await page.waitForFunction((count) => window.responseSaves.length === count, saveCount);
        assert.deepEqual(await page.evaluate(() => window.responseSaves.at(-1)), {
          kind: "matching",
          matches: choice === null ? [] : [{ prompt: "prompt-mito", choice }],
        });
      }

      await activate(energy);
      await activate(slot);
      assert.equal(await slot.innerText(), "Makes energy");
      assert.equal(await bank.isVisible(), true);
      assert.equal(await energy.isDisabled(), true);
      assert.match(await energy.innerText(), /Used in 1 slot/u);
      assert.equal(await protein.isEnabled(), true);
      await assertSaved("choice-energy");

      await activate(protein);
      await activate(slot);
      assert.equal(await slot.innerText(), "Builds proteins");
      assert.equal(await energy.isEnabled(), true);
      assert.equal(await protein.isDisabled(), true);
      await assertSaved("choice-protein");

      await activate(clear);
      assert.match(await slot.getAttribute("aria-label"), /Unanswered/u);
      assert.equal(await energy.isEnabled(), true);
      assert.equal(await protein.isEnabled(), true);
      assert.equal(await bank.isVisible(), true);
      await assertSaved(null);
    } finally {
      await browser.close();
    }
  });
}
