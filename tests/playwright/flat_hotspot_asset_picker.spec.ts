// flat_hotspot_asset_picker.spec.ts - visible keyboard proof for private hotspot image selection.

import { expect, test } from "@playwright/test";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

declare global {
  interface Window {
    __flatHotspotAssetPicker: { readonly selected: ReadonlyArray<string> };
  }
}

let fixtureScript = "";

test.beforeAll(async () => {
  const result = await build({
    bundle: true,
    format: "iife",
    minify: false,
    platform: "browser",
    plugins: [solidPlugin()],
    stdin: {
      contents: `
        import { createComponent } from "solid-js";
        import { render } from "solid-js/web";
        import { createFlatQuestionAssetClient } from "./src/features/flat_question_authoring/flat_question_asset_client.ts";
        import { FlatHotspotAssetPicker } from "./src/features/flat_question_authoring/flat_hotspot_asset_picker.tsx";

        const workspace = "00000000-0000-4000-8000-000000000010";
        const asset = {
          assetId: "aaaaaaaa-0000-4000-8000-000000000011",
          contentChecksum: "a".repeat(64), displayLabel: "Cell membrane diagram",
          mediaType: "image/png", intrinsicWidth: 800, intrinsicHeight: 600,
        };
        const selected = [];
        const json = (value, status = 200) => new Response(JSON.stringify(value), {
          status, headers: { "cache-control": "no-store", "content-type": "application/json" },
        });
        const client = createFlatQuestionAssetClient({
          fetch: async (_input, init = {}) => init.method === "POST" ? json(asset, 201) : json([asset]),
        });
        render(() => createComponent(FlatHotspotAssetPicker, {
          client, workspace, provenance: "Instructor-created diagram",
          onSelect: (descriptor) => selected.push(descriptor.displayLabel),
        }), document.body);
        window.__flatHotspotAssetPicker = { selected };
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "flat_hotspot_asset_picker_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles[0];
  if (output === undefined)
    throw new Error("Flat hotspot asset picker fixture bundle was not produced.");
  fixtureScript = output.text;
});

test("author selects an immutable image through the native keyboard surface without storage leakage", async ({
  page,
}) => {
  await page.goto("/");
  await page.addScriptTag({ content: fixtureScript });

  const picker = page.getByRole("group", { name: "Image" });
  const imageSelect = picker.getByRole("combobox", { name: "Image" });
  await expect(imageSelect).toHaveValue("");
  await expect(picker).toContainText("Cell membrane diagram (800 by 600)");

  await imageSelect.focus();
  await expect(imageSelect).toBeFocused();
  await imageSelect.selectOption("aaaaaaaa-0000-4000-8000-000000000011");
  await expect(imageSelect).toHaveValue("aaaaaaaa-0000-4000-8000-000000000011");
  await expect(picker.getByRole("status")).toContainText("Selected Cell membrane diagram.");
  await expect
    .poll(() => page.evaluate(() => window.__flatHotspotAssetPicker.selected))
    .toEqual(["Cell membrane diagram"]);
  await expect(picker).not.toContainText("aaaaaaaa-0000-4000-8000-000000000011");
  await expect(picker).not.toContainText(
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  );
});
