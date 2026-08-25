import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, statSync, symlinkSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  captureRealStackScreenshot,
  validateVisibleScreenshotText,
} from "./playwright/e2e/real_stack_screenshot_capture.ts";
import { CORPUS_VIEWPORT_SIZES, UI_CORPUS_MANIFEST } from "./playwright/ui_corpus_manifest.ts";

const baseUrl = "https://localhost:55123/";
function captureArtifactsFor(scenarioId) {
  return UI_CORPUS_MANIFEST.filter((artifact) => artifact.scenarioId === scenarioId).map(
    ({ artifactId, stateId }) => ({ artifactId, stateId }),
  );
}

const input = {
  schemaVersion: 2,
  scenarioId: "learner_delivery",
  namespace: "bs1-0123456789ab-learner_delivery",
  baseUrl,
  personas: ["mary_student"],
  baselineReads: ["base_course"],
  visibleObservation: "learner_delivery",
  screenshotCapture: {
    version: 1,
    artifacts: captureArtifactsFor("learner_delivery"),
  },
};

function stagingDirectory() {
  const directory = mkdtempSync(path.join(os.tmpdir(), "ple-screenshot-capture-"));
  mkdirSync(path.join(directory, "staging"), { mode: 0o700 });
  return { root: directory, staging: path.join(directory, "staging") };
}

function page(visible = "Choose one response", feedbackPanels = 0) {
  let viewport = { ...CORPUS_VIEWPORT_SIZES.laptop };
  return {
    viewportSize() {
      return { width: viewport.width, height: viewport.height };
    },
    async setViewportSize(size) {
      viewport = { ...size };
    },
    url() {
      return "https://localhost:55123/assignments/one";
    },
    async evaluate(expression) {
      if (typeof expression === "function" && expression.toString().includes("devicePixelRatio")) {
        return 1;
      }
      return undefined;
    },
    locator(selector) {
      if (selector === "body") return { innerText: async () => visible };
      if (selector === ".feedback-panel") return { count: async () => feedbackPanels };
      if (selector === "body *") return { evaluateAll: async () => 1 };
      throw new Error(`unexpected test locator: ${selector}`);
    },
    async emulateMedia() {},
    async addStyleTag() {},
    async screenshot() {
      return Buffer.from("private screenshot", "ascii");
    },
  };
}

async function withStaging(staging, callback) {
  const prior = process.env.PLE_BROWSER_SUITE_SCREENSHOT_STAGING;
  process.env.PLE_BROWSER_SUITE_SCREENSHOT_STAGING = staging;
  try {
    await callback();
  } finally {
    if (prior === undefined) delete process.env.PLE_BROWSER_SUITE_SCREENSHOT_STAGING;
    else process.env.PLE_BROWSER_SUITE_SCREENSHOT_STAGING = prior;
  }
}

test("real capture writes an exact private receipt with owner-selected file names", async () => {
  const { root, staging } = stagingDirectory();
  try {
    await withStaging(staging, async () => {
      await captureRealStackScreenshot(
        page("Feedback and scores are available according to the assignment settings."),
        input,
        "learner_delivery_problem_ready",
      );
    });
    const png = path.join(staging, "learner_delivery_problem_ready.png");
    const receipt = path.join(staging, "learner_delivery_problem_ready.json");
    assert.equal(statSync(png).mode & 0o777, 0o600);
    assert.equal(statSync(receipt).mode & 0o777, 0o600);
    assert.deepEqual(JSON.parse(readFileSync(receipt, "ascii")), {
      artifactId: "learner_delivery_problem_ready",
      scenarioId: "learner_delivery",
      stateId: "problem_ready",
      sha256: "b6f911247a382851ec1b11f16ab1936274162fdea67c58fa3d854827085c11c6",
      viewport: "laptop",
      width: 1280,
      height: 800,
      origin: "https://localhost:55123",
      privacyValidated: true,
      privacyChecks: ["no_private_material", "no_feedback"],
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("real capture rejects missing, unsafe, duplicate, and undeclared capture requests", async () => {
  const { root, staging } = stagingDirectory();
  try {
    await assert.rejects(() =>
      captureRealStackScreenshot(page(), input, "learner_delivery_problem_ready"),
    );
    await withStaging(staging, async () => {
      await captureRealStackScreenshot(page(), input, "learner_delivery_problem_ready");
      await assert.rejects(() =>
        captureRealStackScreenshot(page(), input, "learner_delivery_problem_ready"),
      );
      await assert.rejects(() => captureRealStackScreenshot(page(), input, "unknown_artifact"));
      await assert.rejects(() =>
        captureRealStackScreenshot(
          page(),
          {
            ...input,
            screenshotCapture: {
              version: 1,
              artifacts: [{ artifactId: "unknown", stateId: "problem_ready" }],
            },
          },
          "learner_delivery_problem_ready",
        ),
      );
    });
    const unsafe = path.join(root, "unsafe");
    mkdirSync(unsafe, { mode: 0o755 });
    await withStaging(unsafe, async () => {
      await assert.rejects(() =>
        captureRealStackScreenshot(page(), input, "learner_delivery_problem_ready"),
      );
    });
    const link = path.join(root, "staging-link");
    symlinkSync(staging, link);
    await withStaging(link, async () => {
      await assert.rejects(() =>
        captureRealStackScreenshot(page(), input, "learner_delivery_problem_ready"),
      );
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("real capture has a visible privacy oracle without broad ordinary-text rejection", async () => {
  validateVisibleScreenshotText("Workspace navigation and a selected response are visible.");
  const privateSamples = [
    "/private/tmp/ple/receipt.json",
    "/Users/example/repository/.git",
    "/tmp/ple/receipt.json",
    "/workspace/ple/receipt.json",
    "PLE_BROWSER_SUITE_SCREENSHOT_STAGING=/private/tmp",
    "Authorization: Bearer secret-value",
    "Cookie: session=secret",
    "Bearer secret-value",
    "Request headers",
    "capability manifest ownership proof",
    "private capability file",
    "disposable manifest path",
    "Invitation link: https://localhost/invite/token",
    "diagnostic overlay",
    "mary.okafor@live-demo.ple.example",
  ];
  for (const sample of privateSamples) {
    assert.throws(() => validateVisibleScreenshotText(sample));
  }
  for (const sample of [
    "Feedback",
    "Correct",
    "Incorrect",
    "Solution",
    "Answer key",
    "Registered passkey",
    "Capability Algorithmic",
    "Question manifest",
  ]) {
    validateVisibleScreenshotText(sample);
  }
  const { root, staging } = stagingDirectory();
  try {
    await withStaging(staging, async () => {
      await assert.rejects(
        () =>
          captureRealStackScreenshot(
            page("ordinary text", 1),
            input,
            "learner_delivery_problem_ready",
          ),
        /evaluation surface/u,
      );
    });
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("ordinary focused journeys remain inert without a screenshot request", async () => {
  const ordinary = { ...input };
  delete ordinary.screenshotCapture;
  await captureRealStackScreenshot(page("Feedback Correct"), ordinary, "anything");
});

test("email-masked capture requires a visible email element and records its privacy treatment", async () => {
  const { root, staging } = stagingDirectory();
  const instructorInput = {
    ...input,
    scenarioId: "instructor_authoring",
    namespace: "bs1-0123456789ab-instructor_authoring",
    personas: ["elena_instructor"],
    screenshotCapture: {
      version: 1,
      artifacts: captureArtifactsFor("instructor_authoring"),
    },
  };
  try {
    await withStaging(staging, async () => {
      await captureRealStackScreenshot(
        page("mary.okafor@live-demo.ple.example"),
        instructorInput,
        "instructor_authoring_invitation_pending",
      );
    });
    const receipt = JSON.parse(
      readFileSync(path.join(staging, "instructor_authoring_invitation_pending.json"), "ascii"),
    );
    assert.deepEqual(receipt.privacyChecks, ["no_private_material", "email_masked"]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
