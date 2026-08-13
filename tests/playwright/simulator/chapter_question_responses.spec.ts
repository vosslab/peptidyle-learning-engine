// chapter_question_responses.spec.ts - answer-key-free candidate enumeration evidence.

import { expect, test } from "@playwright/test";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

import {
  chooseVisibleResponseCandidate,
  completeVisibleQuestionThroughFeedback,
  expectVisibleResponseControlsCleared,
  submitVisibleResponseCandidate,
} from "./chapter_question_responses";
import { geneticsDiagnosisFromVisibleDescription } from "./genetics_chapter_one_responses";

let nativeMatchingFixtureScript = "";

test("reviewed Genetics descriptions resolve from visible biology clues", () => {
  const cases = [
    ["impairs the growth of bone in the limbs", "Achondroplasia"],
    ["reduced hemoglobin proteins causing severe anemia", "Beta-Thalassemia"],
    ["glands that make mucus and sweat", "Cystic fibrosis"],
    ["muscle wasting that gets worse over time", "Duchenne muscular dystrophy"],
    ["changes in part of the X chromosome", "Fragile X syndrome"],
    ["metabolize the sugar galactose", "Galactosemia"],
    ["blood does not clot properly", "Hemophilia"],
    ["progressive breakdown (degeneration) of nerve cells", "Huntington's disease"],
    ["branched-chain amino acids", "Maple syrup urine disease"],
    ["affects the connective tissue", "Marfan syndrome"],
    ["impaired phenylalanine metabolism", "Phenylketonuria"],
    ["stiff and sticky, long, and rigid cells", "Sickle-cell anemia"],
    ["deficiency of the lysosomal enzyme hexosaminidase A", "Tay-Sachs disease"],
    ["duplications of the UBE3A gene", "Angelman syndrome"],
    ["loss of function of specific genes on chromosome 15", "Prader-Willi syndrome"],
    ["high-pitched cat-like cry", "Cri du chat syndrome"],
    ["chromosome 22 is missing", "DiGeorge syndrome"],
    ["extra chromosome 21", "Down syndrome"],
    ["third copy of chromosome 18", "Edwards syndrome"],
    ["47,XXY", "Klinefelter syndrome"],
    ["material from chromosome 13", "Patau syndrome"],
    ["chromosome 9 and chromosome 22 break and exchange portions", "Philadelphia chromosome"],
    ["47,XXX", "Triple X syndrome"],
    ["monosomy X with a webbed neck", "Turner syndrome"],
    ["chromosome number 11", "WAGR syndrome"],
    ["short arm of chromosome 4", "Wolf-Hirschhorn syndrome"],
  ] as const;

  for (const [description, diagnosis] of cases) {
    expect(geneticsDiagnosisFromVisibleDescription(description)).toBe(diagnosis);
  }
});

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

        const text = (id, markdown) => ({ id, body: [{ kind: "text", markdown }] });
        const root = document.createElement("section");
        root.id = "native-matching-fixture";
        document.body.append(root);
        render(() => createComponent(ResponseWidget, {
          attemptId: "native-matching-attempt",
          definition: {
            kind: "matching",
            prompts: [text("prompt-1", "Prompt one"), text("prompt-2", "Prompt two"), text("prompt-3", "Prompt three"), text("prompt-4", "Prompt four")],
            choices: [text("choice-1", "Choice one"), text("choice-2", "Choice two"), text("choice-3", "Choice three"), text("choice-4", "Choice four")],
          },
          validator: { mode: "wasm", validateResponseFormat: async () => ({ violations: [] }) },
          onSubmit: async () => { root.dataset.submitted = "true"; },
          onEscape: () => {},
        }), root);
      `,
      loader: "tsx",
      resolveDir: process.cwd(),
      sourcefile: "chapter_question_responses_fixture.tsx",
    },
    write: false,
  });
  const output = result.outputFiles[0];
  if (output === undefined) throw new Error("Native matching fixture bundle was not produced.");
  nativeMatchingFixtureScript = output.text;
});

async function mountNativeMatchingFixture(page: import("@playwright/test").Page): Promise<void> {
  await page.goto("/");
  await page.addScriptTag({ content: nativeMatchingFixtureScript });
}

function matchingGroup(values: readonly string[]): string {
  const controls = values
    .map(
      (value) =>
        `<button type="button" role="radio" data-choice-id="${value}" aria-checked="false" aria-disabled="false" tabindex="0" onclick="this.setAttribute('aria-checked', 'true')">Visible ${value}</button>`,
    )
    .join("");
  return `<section class="matching-group" role="group" aria-label="Visible prompt"><div role="radiogroup" aria-label="Visible prompt">${controls}</div></section>`;
}

function shuffledMatchingMarkup(): string {
  return `<main>${matchingGroup(["choice-01", "choice-02", "choice-03"])}${matchingGroup(["choice-02", "choice-01", "choice-03"])}${matchingGroup(["choice-03", "choice-01", "choice-02"])}</main>`;
}

function semanticMatchingGroup(
  prompt: string,
  choices: ReadonlyArray<{ readonly id: string; readonly text: string }>,
): string {
  const controls = choices
    .map(
      ({ id, text }) =>
        `<button type="button" role="radio" data-choice-id="${id}" aria-checked="false" aria-disabled="false" tabindex="0" onclick="this.setAttribute('aria-checked', 'true')"><span class="matching-choice-content"><span>${text}</span></span></button>`,
    )
    .join("");
  return `<section class="matching-group" role="group" aria-label="${prompt}"><p>${prompt}</p>${controls}</section>`;
}

async function expectSemanticCandidateZero(page: import("@playwright/test").Page): Promise<void> {
  const alpha = page.locator(".matching-group").filter({
    has: page.getByText("Alpha prompt", { exact: true }),
  });
  const beta = page.locator(".matching-group").filter({
    has: page.getByText("Beta prompt", { exact: true }),
  });
  await expect(alpha.getByRole("radio", { name: "Cat" })).toHaveAttribute("aria-checked", "true");
  await expect(beta.getByRole("radio", { name: "Dog" })).toHaveAttribute("aria-checked", "true");
}

test("matching candidates use opaque identities when every prompt shuffles visible choices", async ({
  page,
}) => {
  await page.setContent(shuffledMatchingMarkup());

  await expect(chooseVisibleResponseCandidate(page, 0)).resolves.toBe("matching");
  const groups = page.locator(".matching-group");
  await expect(groups.nth(0).locator('[data-choice-id="choice-01"]')).toHaveAttribute(
    "aria-checked",
    "true",
  );
  await expect(groups.nth(1).locator('[data-choice-id="choice-02"]')).toHaveAttribute(
    "aria-checked",
    "true",
  );
  await expect(groups.nth(2).locator('[data-choice-id="choice-03"]')).toHaveAttribute(
    "aria-checked",
    "true",
  );

  await page.setContent(shuffledMatchingMarkup());
  await expect(chooseVisibleResponseCandidate(page, 1)).resolves.toBe("matching");
  await expect(groups.nth(0).locator('[data-choice-id="choice-02"]')).toHaveAttribute(
    "aria-checked",
    "true",
  );
  await expect(groups.nth(1).locator('[data-choice-id="choice-01"]')).toHaveAttribute(
    "aria-checked",
    "true",
  );
  await expect(groups.nth(2).locator('[data-choice-id="choice-03"]')).toHaveAttribute(
    "aria-checked",
    "true",
  );
});

test("matching candidate ordinals retain visible meaning across fresh rendered IDs", async ({
  page,
}) => {
  await page.setContent(
    `<main>${semanticMatchingGroup("Beta prompt", [
      { id: "old-dog", text: "Dog" },
      { id: "old-cat", text: "Cat" },
    ])}${semanticMatchingGroup("Alpha prompt", [
      { id: "old-cat", text: "Cat" },
      { id: "old-dog", text: "Dog" },
    ])}</main>`,
  );
  await expect(chooseVisibleResponseCandidate(page, 0)).resolves.toBe("matching");
  await expectSemanticCandidateZero(page);

  await page.setContent(
    `<main>${semanticMatchingGroup("Alpha prompt", [
      { id: "new-dog", text: "Dog" },
      { id: "new-cat", text: "Cat" },
    ])}${semanticMatchingGroup("Beta prompt", [
      { id: "new-cat", text: "Cat" },
      { id: "new-dog", text: "Dog" },
    ])}</main>`,
  );
  await expect(chooseVisibleResponseCandidate(page, 0)).resolves.toBe("matching");
  await expectSemanticCandidateZero(page);
});

test("native four-prompt matching makes candidate one ready through Tab and Space", async ({
  page,
}) => {
  test.setTimeout(2_000);
  await mountNativeMatchingFixture(page);
  const fixture = page.locator("#native-matching-fixture");

  await expect(expectVisibleResponseControlsCleared(page)).resolves.toBeUndefined();
  await expect(chooseVisibleResponseCandidate(page, 1)).resolves.toBe("matching");
  await expect(fixture.getByRole("status", { name: "Response format" })).toContainText(
    "ready to submit",
  );
  const submit = fixture.getByRole("button", { name: "Submit answer" });
  await submit.press("Space");
  await expect(fixture).toHaveAttribute("data-submitted", "true");
});

test("cleared-response proof waits for a server-issued matching question", async ({ page }) => {
  await page.setContent('<main id="next-question"><p>Loading the next question...</p></main>');
  await page.locator("#next-question").evaluate(
    (container, markup) => {
      globalThis.setTimeout(() => {
        container.innerHTML = markup;
      }, 50);
    },
    matchingGroup(["choice-01", "choice-02"]),
  );

  await expect(expectVisibleResponseControlsCleared(page)).resolves.toBeUndefined();
});

test("complete response waits through a mounted loading shell before detecting the family", async ({
  page,
}) => {
  await page.setContent(
    '<main data-route-surface="runAttempt">Loading the next question...</main>',
  );
  await page.locator("main").evaluate((container) => {
    globalThis.setTimeout(() => {
      container.innerHTML = `
        <label><input type="radio" name="response" value="one">Visible option one</label>
        <label><input type="radio" name="response" value="two">Visible option two</label>
        <p role="status" aria-label="Response format">ready to submit</p>
        <button type="button" onclick="document.getElementById('feedback').hidden = false">Submit answer</button>
        <section id="feedback" role="region" aria-label="Feedback" hidden>
          <h2>Feedback</h2><h3>Your response</h3><h3>Correct</h3>
          <button type="button">Continue</button>
        </section>
      `;
    }, 50);
  });

  await expect(completeVisibleQuestionThroughFeedback(page)).resolves.toBe("multiple-choice");
});

test("response progress distinguishes a complete selection from visible feedback", async ({
  page,
}) => {
  await page.setContent(`
    <label><input type="radio" name="response" value="one" tabindex="0">Visible option</label>
    <label><input type="radio" name="response" value="two" tabindex="0">Visible option</label>
    <p role="status" aria-label="Response format">ready to submit</p>
    <button type="button" onclick="document.getElementById('feedback').hidden = false">Submit answer</button>
    <section id="feedback" role="region" aria-label="Feedback" hidden>
      <h2>Feedback</h2><h3>Your response</h3><h3>Correct</h3>
    </section>
  `);
  const stages: string[] = [];
  await expect(page.getByRole("button", { name: "Submit answer" })).toBeVisible();

  await submitVisibleResponseCandidate(page, 1, {
    responseSelected: () => stages.push("response_selected"),
    feedbackVisible: () => stages.push("feedback_visible"),
  });

  await expect(page.locator('input[type="radio"]').nth(1)).toBeChecked();
  expect(stages).toEqual(["response_selected", "feedback_visible"]);
});
