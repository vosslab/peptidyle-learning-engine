// Stable browser boundary checks for a backend-owned document bridge.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import test from "node:test";

import {
  backendOwnedDocumentPath,
  backendOwnedResponseFromPairs,
  classifyBackendOwnedResponseMessage,
  isBackendOwnedResponseMessage,
} from "../src/components/question_response_controls/backend_owned_document.tsx";
import {
  saveCapturedBackendOwnedResponse,
  saveCompleteResponseBeforeAttemptSubmission,
} from "../src/pages/assessment_attempt_finish.ts";

const bridgeSource = readFileSync(new URL("../src/public/ple_bridge.js", import.meta.url), "utf8");

function loadBridge(entries = []) {
  const documentListeners = new Map();
  const windowListeners = new Map();
  const sent = [];
  const parent = { postMessage: (message, origin) => sent.push({ message, origin }) };
  class HTMLFormElement {}
  const form = new HTMLFormElement();
  const context = {
    Array,
    FormData: class {
      constructor(received) {
        assert.equal(received, form);
        return entries;
      }
    },
    HTMLFormElement,
    Object,
    String,
    document: {
      addEventListener: (name, listener) => documentListeners.set(name, listener),
      querySelector: (selector) => {
        assert.equal(selector, "form");
        return form;
      },
    },
    window: {
      location: { origin: "https://ple.test" },
      parent,
      addEventListener: (name, listener) => windowListeners.set(name, listener),
    },
  };
  vm.runInNewContext(bridgeSource, context, { filename: "ple_bridge.js" });
  return { documentListeners, form, parent, sent, windowListeners };
}

test("bridge captures an entire minimal form in order, including duplicate and hidden values", () => {
  const bridge = loadBridge([
    ["AnSwEr0001", "first"],
    ["AnSwEr0001", "second"],
    ["legitimate_hidden", "state"],
  ]);
  const submit = bridge.documentListeners.get("submit");
  let prevented = false;

  submit({ preventDefault: () => (prevented = true), target: bridge.form });

  assert.equal(prevented, true);
  assert.deepEqual(JSON.parse(JSON.stringify(bridge.sent)), [
    { message: { kind: "ple.backendOwned.ready" }, origin: "https://ple.test" },
    {
      message: {
        kind: "ple.backendOwned.response",
        pairs: [
          ["AnSwEr0001", "first"],
          ["AnSwEr0001", "second"],
          ["legitimate_hidden", "state"],
        ],
      },
      origin: "https://ple.test",
    },
  ]);
});

test("bridge accepts a strict string capture request only from its exact parent and origin", () => {
  const bridge = loadBridge([["answer", "saved"]]);
  const receive = bridge.windowListeners.get("message");

  receive({
    data: "ple.backendOwned.capture:0123456789abcdef",
    origin: "https://other.test",
    source: bridge.parent,
  });
  receive({
    data: "ple.backendOwned.capture:0123456789abcdef",
    origin: "https://ple.test",
    source: {},
  });
  receive({
    data: "ple.backendOwned.capture:ABCDEF0123456789",
    origin: "https://ple.test",
    source: bridge.parent,
  });
  receive({
    data: "ple.backendOwned.capture:0123456789abcdef:extra",
    origin: "https://ple.test",
    source: bridge.parent,
  });
  receive({
    data: { kind: "ple.backendOwned.capture", captureId: "0123456789abcdef" },
    origin: "https://ple.test",
    source: bridge.parent,
  });
  assert.equal(bridge.sent.length, 1);

  receive({
    data: "ple.backendOwned.capture:0123456789abcdef",
    origin: "https://ple.test",
    source: bridge.parent,
  });
  assert.deepEqual(JSON.parse(JSON.stringify(bridge.sent.at(-1))), {
    message: {
      kind: "ple.backendOwned.response",
      pairs: [["answer", "saved"]],
      captureId: "0123456789abcdef",
    },
    origin: "https://ple.test",
  });
});

test("browser response capture shares the backend's raw 64 KiB limit", () => {
  const emptyValueBytes = new TextEncoder().encode(JSON.stringify([["answer", ""]])).length;
  const pairsAt = (byteLength) => [["answer", "x".repeat(byteLength - emptyValueBytes)]];

  for (const byteLength of [65_536, 65_537]) {
    const pairs = pairsAt(byteLength);
    assert.equal(new TextEncoder().encode(JSON.stringify(pairs)).length, byteLength);
    assert.equal(backendOwnedResponseFromPairs(pairs) !== null, byteLength === 65_536);
  }
});

test("only the matching capture reply can advance Finish past queued bridge submissions", () => {
  const captureId = "0123456789abcdef";
  const queued = [
    { kind: "ple.backendOwned.response", pairs: [["answer", "ordinary"]] },
    {
      kind: "ple.backendOwned.response",
      pairs: [["answer", "wrong"]],
      captureId: "fedcba9876543210",
    },
    {
      kind: "ple.backendOwned.response",
      pairs: [["answer", "stale"]],
      captureId: "aaaaaaaaaaaaaaaa",
    },
    {
      kind: "ple.backendOwned.response",
      pairs: [["answer", "current"]],
      captureId,
    },
  ];
  const events = [];
  let captured;

  for (const message of queued) {
    const disposition = classifyBackendOwnedResponseMessage(message, captureId);
    if (disposition?.kind === "ordinary") events.push("ordinary response");
    if (disposition?.kind === "capture") captured = disposition.pairs;
  }
  if (captured !== undefined) events.push("capture", "save", "finalize");

  assert.deepEqual(captured, [["answer", "current"]]);
  assert.deepEqual(events, ["ordinary response", "capture", "save", "finalize"]);
});

test("parent accepts only the bridge message and builds only the current document route", () => {
  assert.equal(
    backendOwnedDocumentPath("R-42", 3),
    "/api/assessment-attempts/R-42/questions/3/document",
  );
  assert.equal(backendOwnedDocumentPath("question-attempt-42", 3), null);
  assert.equal(backendOwnedDocumentPath("R-42", 0), null);
  assert.equal(
    isBackendOwnedResponseMessage({
      kind: "ple.backendOwned.response",
      pairs: [
        ["duplicate", "one"],
        ["duplicate", "two"],
      ],
    }),
    true,
  );
  assert.equal(
    isBackendOwnedResponseMessage({ kind: "ple.backendOwned.response", pairs: [], extra: true }),
    false,
  );
});

test("Finish Assessment captures the active backend document, saves it, then finalizes", async () => {
  const events = [];
  const result = await saveCapturedBackendOwnedResponse(
    async () => {
      events.push("capture");
      return true;
    },
    async () => {
      events.push("save");
      return true;
    },
    async () => {
      events.push("finalize");
      return { submitted: true };
    },
  );

  assert.deepEqual(result, { submitted: true });
  assert.deepEqual(events, ["capture", "save", "finalize"]);
});

test("whole-Attempt submission skips incomplete local input but blocks on a complete-response save failure", async () => {
  let saveCalls = 0;
  const save = async () => {
    saveCalls += 1;
    return false;
  };

  assert.equal(await saveCompleteResponseBeforeAttemptSubmission(false, save), true);
  assert.equal(saveCalls, 0);
  assert.equal(await saveCompleteResponseBeforeAttemptSubmission(true, save), false);
  assert.equal(saveCalls, 1);
});

test("incomplete local input preserves an earlier saved response and awaits a pending save failure", async () => {
  let saveCalls = 0;
  const save = async () => {
    saveCalls += 1;
    return true;
  };

  assert.equal(
    await saveCompleteResponseBeforeAttemptSubmission(false, save, Promise.resolve(false)),
    false,
  );
  assert.equal(saveCalls, 0, "the incomplete draft does not replace the earlier saved response");
  assert.equal(
    await saveCompleteResponseBeforeAttemptSubmission(false, save, Promise.resolve(true)),
    true,
  );
  assert.equal(saveCalls, 0, "the incomplete draft remains unsaved before whole submission");
});
