// e2e_ribbon_destination_ledger_contract.mjs - generator integration contracts.

import assert from "node:assert/strict";
import test from "node:test";

import {
  evidenceLink,
  generatedEnd,
  generatedStart,
  main,
  replaceGeneratedSection,
  renderGeneratedLedger,
} from "../../devel/generate_ribbon_destination_ledger.mjs";

const preservationTestName = [
  "the destination-ledger generator preserves human-owned prose",
  "and rejects alternate outputs",
].join(" ");

test("ledger evidence links keep symbols outside path-matching link text", () => {
  assert.equal(
    evidenceLink("src/api/application_api.tsx::ApiClient.listCourses"),
    "[src/api/application_api.tsx](../../src/api/application_api.tsx)::ApiClient.listCourses",
  );
  assert.equal(
    evidenceLink("src/route_contract.ts"),
    "[src/route_contract.ts](../../src/route_contract.ts)",
  );
});

test(preservationTestName, () => {
  const example = [
    generatedStart,
    "old machine data",
    generatedEnd,
    "",
    "## Editorial",
    "",
    "Human prose.",
  ].join("\n");
  const replacement = replaceGeneratedSection(example, renderGeneratedLedger());
  assert.match(replacement, /## Editorial\n\nHuman prose\./u);
  assert.throws(() => main(["--output", "/tmp/other.md"]), /Usage:/u);
});

test("the destination-ledger generator rejects malformed generated-section markers", () => {
  for (const documentText of [
    `${generatedStart}\nold\n${generatedStart}\n${generatedEnd}`,
    `${generatedStart}\nold\n${generatedEnd}\n${generatedEnd}`,
    `${generatedEnd}\nold\n${generatedStart}`,
    `${generatedStart}\nold`,
    `old\n${generatedEnd}`,
  ]) {
    assert.throws(
      () => replaceGeneratedSection(documentText, renderGeneratedLedger()),
      /markers are missing or out of order/u,
    );
  }
});
