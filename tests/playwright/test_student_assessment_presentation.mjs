// Shared answer-free Student landing behavior checks.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { chromium } from "playwright";
import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";

const startSummaryCss = readFileSync(
  new URL("../../src/components/student_assessment_presentation.css", import.meta.url),
  "utf8",
);
const pageFrameCss = readFileSync(
  new URL("../../src/components/page_frame.css", import.meta.url),
  "utf8",
);
const STUDENT_LAYOUT_SENTENCE =
  "Student layouts should adapt smoothly at intermediate widths, with readable long titles and controls that wrap or rearrange in the task's reading order.";
const LONG_COURSEWORK_TITLE =
  "Weekly assignment phenylalaninephenylalaninephenylalaninephenylalaninephenylalaninephenylalaninephenylalaninephenylalaninephenylalaninephenylalaninephenylalaninephenylalanine";

async function loadDecisionDetailsForSsr() {
  const result = await build({
    bundle: true,
    entryPoints: [
      new URL("../../src/components/student_assessment_presentation.tsx", import.meta.url).pathname,
    ],
    format: "esm",
    outfile: "student_assessment_presentation.js",
    platform: "node",
    plugins: [solidPlugin({ solid: { generate: "ssr", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined)
    throw new Error("Student Assessment decision SSR bundle is missing.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module.StudentAssessmentDecisionDetails !== "function") {
    throw new Error("Student Assessment decision component export is missing.");
  }
  return module;
}

test("Student detail adapts available entries and Question Pool selections without exposing source identities", async () => {
  const { toStudentAssessmentPresentationData } = await loadDecisionDetailsForSsr();
  const presentation = toStudentAssessmentPresentationData({
    id: "A7K3M2QAS",
    title: "Protein structure",
    instructions: "Use your notes.",
    display_time_zone: "America/New_York",
    delivery: {
      available_at: null,
      due_at: null,
      closes_at: null,
      assessment_attempt_time_limit_seconds: 900,
      attempt_limit: 2,
      late_work_rule: "accept",
      student_late_work_status: "on_time",
    },
    entries: [
      { kind: "fixedQuestion", availability: "available" },
      { kind: "fixedQuestion", availability: "retired" },
      { kind: "fixedQuestion", availability: "available" },
      { kind: "questionPool", availability: "available", selectionCount: 3 },
      { kind: "questionPool", availability: "retired", selectionCount: 2 },
    ],
  });

  assert.equal(presentation.questionsPerAssessmentAttempt, 5);
  assert.equal(presentation.delivery.studentLateWorkStatus, "on_time");
  assert.equal(presentation.displayTimeZone, "America/New_York");
  assert.equal("timeZone" in presentation, false);
  assert.equal("id" in presentation, false);
});

test("base duration defaults calculate from Questions while explicit and legacy durations remain stored", async () => {
  const { formatAssessmentAttemptTimeLimit } = await loadDecisionDetailsForSsr();
  assert.equal(formatAssessmentAttemptTimeLimit(null, 3), "5 minutes per attempt");
  assert.equal(formatAssessmentAttemptTimeLimit(3_600), "1 hour per attempt");
  assert.equal(formatAssessmentAttemptTimeLimit(90), "90 seconds per attempt");
});

test("assessment instants use the supplied viewer zone instead of the browser zone", async () => {
  const { formatAssessmentActivity, formatAssessmentDeliveryTime } =
    await loadDecisionDetailsForSsr();
  const timestamp = Date.parse("2026-01-15T18:30:00Z");
  const newYork = "America/New_York";
  const losAngeles = "America/Los_Angeles";
  const newYorkDateTimeFormatter = new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: newYork,
  });
  const losAngelesDateTimeFormatter = new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: losAngeles,
  });
  const newYorkFormatter = (timestamp) => newYorkDateTimeFormatter.format(timestamp);
  const losAngelesFormatter = (timestamp) => losAngelesDateTimeFormatter.format(timestamp);
  const expected = newYorkDateTimeFormatter.format(new Date(timestamp));

  assert.equal(formatAssessmentDeliveryTime(timestamp, newYorkFormatter), expected);
  assert.equal(formatAssessmentActivity(timestamp, newYorkFormatter), expected);
  assert.equal(formatAssessmentDeliveryTime(null, newYorkFormatter, "No due time"), "No due time");
  assert.equal(formatAssessmentActivity(null, newYorkFormatter), "No activity yet");
  assert.notEqual(
    formatAssessmentDeliveryTime(timestamp, losAngelesFormatter),
    expected,
    "fixed instant must render in the supplied viewer zone",
  );
});

test("decision presentation uses the supplied zone without printing its name", async () => {
  const { StudentAssessmentDecisionDetails, formatAssessmentDeliveryTime } =
    await loadDecisionDetailsForSsr();
  const dueAt = Date.parse("2026-01-15T18:30:00Z");
  const newYorkDateTimeFormatter = new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "America/New_York",
  });
  const losAngelesDateTimeFormatter = new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "America/Los_Angeles",
  });
  const newYorkFormatter = (timestamp) => newYorkDateTimeFormatter.format(timestamp);
  const losAngelesFormatter = (timestamp) => losAngelesDateTimeFormatter.format(timestamp);
  const decision = {
    availableAt: Date.parse("2026-01-15T17:30:00Z"),
    dueAt,
    closesAt: Date.parse("2026-01-15T19:30:00Z"),
    timeLimitSeconds: 900,
    attemptLimit: 2,
    lateWorkRule: "reject",
    displayTimeZone: "America/New_York",
    evaluatedAt: Date.parse("2026-01-15T17:00:00Z"),
    startDecision: "may_start",
    publicReason: null,
  };
  const newYorkDue = formatAssessmentDeliveryTime(dueAt, newYorkFormatter);
  const losAngelesDue = formatAssessmentDeliveryTime(dueAt, losAngelesFormatter);
  const newYorkHtml = renderToString(() =>
    createComponent(StudentAssessmentDecisionDetails, { decision }),
  );
  const losAngelesHtml = renderToString(() =>
    createComponent(StudentAssessmentDecisionDetails, {
      decision: { ...decision, displayTimeZone: "America/Los_Angeles" },
    }),
  );

  assert.notEqual(newYorkDue, losAngelesDue);
  assert.ok(newYorkHtml.includes(newYorkDue));
  assert.ok(losAngelesHtml.includes(losAngelesDue));
  assert.match(newYorkHtml, /Can start/u);
  assert.doesNotMatch(newYorkHtml, /America\/New_York|time zone/iu);
  assert.doesNotMatch(losAngelesHtml, /America\/Los_Angeles|time zone/iu);
  assert.doesNotMatch(newYorkHtml, /Cannot start/u);

  const closedReason = "This Coursework is closed for new work.";
  const closedHtml = renderToString(() =>
    createComponent(StudentAssessmentDecisionDetails, {
      decision: { ...decision, startDecision: "closed", publicReason: closedReason },
    }),
  );
  assert.match(closedHtml, /Cannot start/u);
  assert.equal((closedHtml.match(new RegExp(closedReason, "gu")) ?? []).length, 1);
});

test("unset Coursework limits use Student language in the selected display zone", async () => {
  const { StudentAssessmentDecisionDetails, formatAssessmentDeliveryTime } =
    await loadDecisionDetailsForSsr();
  const dueAt = Date.parse("2026-01-15T18:30:00Z");
  const chicagoFormatter = new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "America/Chicago",
  });
  const newYorkFormatter = new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
    timeZone: "America/New_York",
  });
  const decision = {
    availableAt: null,
    dueAt,
    closesAt: null,
    timeLimitSeconds: null,
    attemptLimit: null,
    lateWorkRule: "accept",
    displayTimeZone: "America/Chicago",
    evaluatedAt: dueAt,
    startDecision: "may_start",
    publicReason: null,
  };
  const chicagoDue = formatAssessmentDeliveryTime(dueAt, (timestamp) =>
    chicagoFormatter.format(timestamp),
  );
  const newYorkDue = formatAssessmentDeliveryTime(dueAt, (timestamp) =>
    newYorkFormatter.format(timestamp),
  );
  const chicagoHtml = renderToString(() =>
    createComponent(StudentAssessmentDecisionDetails, { decision }),
  );
  const newYorkHtml = renderToString(() =>
    createComponent(StudentAssessmentDecisionDetails, {
      decision: { ...decision, displayTimeZone: "America/New_York" },
    }),
  );

  assert.notEqual(chicagoDue, newYorkDue);
  assert.ok(chicagoHtml.includes(chicagoDue));
  assert.ok(newYorkHtml.includes(newYorkDue));
  assert.ok(!chicagoHtml.includes(newYorkDue));
  assert.match(chicagoHtml, /No closing time/u);
  assert.match(chicagoHtml, /Unlimited Attempts/u);
  assert.match(newYorkHtml, /No closing time/u);
  assert.match(newYorkHtml, /Unlimited Attempts/u);
  assert.doesNotMatch(chicagoHtml, /America\/Chicago/u);
  assert.doesNotMatch(newYorkHtml, /America\/New_York/u);
});

function startSummaryDocument(summaryHtml) {
  return `<!doctype html>
<html>
<head>
  <style>
    :root {
      --ple-border: #718096;
      --ple-muted: #4a5568;
      --ple-space-2: 0.5rem;
      --ple-space-3: 0.75rem;
      --ple-space-5: 1.25rem;
    }
    body { margin: 0; }
  </style>
  <style>${startSummaryCss}</style>
</head>
<body>${summaryHtml}</body>
</html>`;
}

function startSummaryDecision() {
  return {
    availableAt: Date.parse("2026-01-15T17:30:00Z"),
    dueAt: Date.parse("2026-01-15T18:30:00Z"),
    closesAt: Date.parse("2026-01-15T19:30:00Z"),
    timeLimitSeconds: 900,
    attemptLimit: 3,
    lateWorkRule: "accept",
    displayTimeZone: "America/Chicago",
    evaluatedAt: Date.parse("2026-01-15T17:00:00Z"),
    startDecision: "may_start",
    publicReason: null,
  };
}

async function measureStartSummary(page) {
  return page.evaluate(() => {
    function trackCount(element) {
      const value = getComputedStyle(element).gridTemplateColumns.trim();
      if (value === "" || value === "none") return 0;
      return value.split(/\s+/u).length;
    }
    const section = document.querySelector(".student-assessment-start-facts");
    const lists = [...section.querySelectorAll(".assessment-facts")].map((list) => ({
      className: list.className,
      columns: trackCount(list),
      labels: [...list.querySelectorAll(":scope > div > dt")].map((node) => node.textContent),
    }));
    const pairs = [...section.querySelectorAll(".assessment-facts > div")]
      .filter((row) => {
        const details = row.closest("details");
        return details === null || details.open;
      })
      .map((row) => {
        const label = row.querySelector("dt").getBoundingClientRect();
        const value = row.querySelector("dd").getBoundingClientRect();
        const overlap = Math.min(label.bottom, value.bottom) - Math.max(label.top, value.top);
        return {
          label: row.querySelector("dt").textContent,
          columns: trackCount(row),
          labelBesideValue: label.right <= value.left + 1 && overlap > 0,
        };
      });
    return {
      heading: section.querySelector("h2")?.textContent ?? "",
      lists,
      pairs,
    };
  });
}

test("Before you start keeps each label beside its value in a compact grid", async () => {
  const { StudentAssessmentStartFacts } = await loadDecisionDetailsForSsr();
  const summaryHtml = renderToString(() =>
    createComponent(StudentAssessmentStartFacts, {
      questionCount: 4,
      pointsPossible: 12,
      timeLimitSeconds: 900,
      decision: startSummaryDecision(),
    }),
  );
  assert.match(summaryHtml, /Before you start/);
  assert.match(startSummaryCss, /\.student-assessment-start-facts \.assessment-facts > div/);

  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    await page.setContent(startSummaryDocument(summaryHtml), { waitUntil: "load" });
    const wide = await measureStartSummary(page);
    await page.setViewportSize({ width: 320, height: 800 });
    const narrow = await measureStartSummary(page);

    assert.equal(wide.heading, "Before you start");
    assert.ok(wide.pairs.length >= 2);
    for (const pair of [...wide.pairs, ...narrow.pairs]) {
      assert.equal(pair.labelBesideValue, true, pair.label);
      assert.equal(pair.columns, 2, pair.label);
    }
    const wideWork = wide.lists.find((list) => list.className.includes("start-facts__work"));
    const narrowWork = narrow.lists.find((list) => list.className.includes("start-facts__work"));
    assert.deepEqual(wideWork.labels, ["Questions", "Points possible"]);
    assert.ok(wideWork.columns >= 2);
    assert.equal(narrowWork.columns, 1);
    const wideAttemptRules = wide.lists.find(
      (list) => list.labels.includes("Time limit") && list.labels.includes("Attempt limit"),
    );
    const narrowAttemptRules = narrow.lists.find(
      (list) => list.labels.includes("Time limit") && list.labels.includes("Attempt limit"),
    );
    assert.ok(wideAttemptRules.columns >= 2);
    assert.equal(narrowAttemptRules.columns, 1);
  } finally {
    await browser.close();
  }
});

async function renderStudentCourseworkLanding() {
  const result = await build({
    bundle: true,
    stdin: {
      contents: `
        import { createComponent } from "solid-js";
        import { renderToString, ssr } from "solid-js/web";
        import { ApplicationApiProvider } from "./src/api/application_api.tsx";
        import { StudentAssessmentPresentation } from "./src/components/student_assessment_presentation.tsx";
        import { RouteScopeProvider } from "./src/ribbon/route_scope_context.tsx";
        const title = ${JSON.stringify(LONG_COURSEWORK_TITLE)};
        export function renderLanding() {
          return renderToString(() =>
            createComponent(ApplicationApiProvider, {
              applicationApi: { client: {}, queries: {} },
              get children() {
                return createComponent(RouteScopeProvider, {
                  pathname: "/",
                  get children() {
                    return createComponent(StudentAssessmentPresentation, {
                      assessment: {
                        title,
                        instructions: "",
                        displayTimeZone: "America/Chicago",
                        questionsPerAssessmentAttempt: 4,
                        delivery: {
                          availableAt: null,
                          dueAt: null,
                          closesAt: null,
                          assessmentAttemptTimeLimitSeconds: 900,
                          attemptLimit: 2,
                          lateWorkRule: "accept",
                        },
                      },
                      get primaryAction() {
                        return ssr(
                          '<button type="button" class="primary-action">Start Weekly Assignment</button>',
                        );
                      },
                      get secondaryAction() {
                        return ssr(
                          '<button type="button" class="quiet-action">Review previous Attempt</button>',
                        );
                      },
                    });
                  },
                });
              },
            }),
          );
        }
      `,
      loader: "js",
      resolveDir: new URL("../..", import.meta.url).pathname,
    },
    format: "esm",
    outfile: "student-coursework-landing.js",
    platform: "node",
    plugins: [
      solidPlugin({ solid: { generate: "ssr", hydratable: false } }),
      {
        name: "css-stub",
        setup(pluginBuild) {
          pluginBuild.onResolve({ filter: /\.css$/ }, (args) => ({
            path: args.path,
            namespace: "css-stub",
          }));
          pluginBuild.onLoad({ filter: /.*/, namespace: "css-stub" }, () => ({
            contents: "export default {};",
            loader: "js",
          }));
        },
      },
    ],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) throw new Error("Student Coursework landing bundle is missing.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  return module.renderLanding();
}

function studentLandingDocument(landingHtml) {
  return `<!doctype html>
<html>
<head>
  <style>
    :root {
      --ple-border: #718096;
      --ple-muted: #4a5568;
      --ple-ink: #1a202c;
      --ple-space-2: 0.5rem;
      --ple-space-3: 0.75rem;
      --ple-space-5: 1.25rem;
      --ple-reading-max-inline: 72rem;
      --ple-section-gap: 0.75rem;
    }
    body { margin: 0; font-family: sans-serif; }
  </style>
  <style>${pageFrameCss}</style>
  <style>${startSummaryCss}</style>
</head>
<body>${landingHtml}</body>
</html>`;
}

async function measureStudentLanding(page) {
  return page.evaluate(() => {
    const heading = document.querySelector("h1.page-frame__title");
    const region = document.querySelector(".student-assessment-action-region");
    const primary = region?.querySelector(".student-assessment-primary-action");
    const secondary = region?.querySelector(".student-assessment-secondary-actions");
    if (!(heading instanceof HTMLElement) || !(primary instanceof HTMLElement)) {
      throw new Error("student landing is missing its title or actions");
    }
    if (!(secondary instanceof HTMLElement)) {
      throw new Error("student landing is missing its secondary action");
    }
    const columns = getComputedStyle(region).gridTemplateColumns.trim().split(/\s+/u).length;
    const titleBox = heading.getBoundingClientRect();
    const primaryBox = primary.getBoundingClientRect();
    const secondaryBox = secondary.getBoundingClientRect();
    const range = document.createRange();
    range.selectNodeContents(heading);
    return {
      title: heading.textContent,
      columns,
      titleRight: titleBox.right,
      titleLineCount: range.getClientRects().length,
      titleFits: heading.scrollWidth <= heading.clientWidth + 1,
      primaryText: primary.textContent.replace(/\s+/gu, " ").trim(),
      secondaryText: secondary.textContent.replace(/\s+/gu, " ").trim(),
      primaryTop: primaryBox.top,
      primaryLeft: primaryBox.left,
      primaryRight: primaryBox.right,
      secondaryTop: secondaryBox.top,
      secondaryLeft: secondaryBox.left,
      secondaryRight: secondaryBox.right,
      viewportWidth: window.innerWidth,
    };
  });
}

test(STUDENT_LAYOUT_SENTENCE, async () => {
  const landingHtml = await renderStudentCourseworkLanding();
  assert.match(landingHtml, new RegExp(LONG_COURSEWORK_TITLE));
  assert.match(landingHtml, /Start Weekly Assignment/);
  assert.match(landingHtml, /Review previous Attempt/);
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    await page.setContent(studentLandingDocument(landingHtml), { waitUntil: "load" });
    const wide = await measureStudentLanding(page);
    await page.setViewportSize({ width: 800, height: 800 });
    const intermediate = await measureStudentLanding(page);
    assert.equal(wide.title, LONG_COURSEWORK_TITLE, STUDENT_LAYOUT_SENTENCE);
    assert.equal(wide.columns, 2, STUDENT_LAYOUT_SENTENCE);
    assert.equal(wide.titleFits, true, STUDENT_LAYOUT_SENTENCE);
    assert.ok(wide.primaryRight <= wide.secondaryLeft + 1, STUDENT_LAYOUT_SENTENCE);
    assert.ok(Math.abs(wide.primaryTop - wide.secondaryTop) < 2, STUDENT_LAYOUT_SENTENCE);
    assert.ok(wide.primaryRight <= wide.viewportWidth + 1, STUDENT_LAYOUT_SENTENCE);
    assert.ok(wide.secondaryRight <= wide.viewportWidth + 1, STUDENT_LAYOUT_SENTENCE);
    assert.equal(intermediate.columns, 1, STUDENT_LAYOUT_SENTENCE);
    assert.equal(intermediate.titleFits, true, STUDENT_LAYOUT_SENTENCE);
    assert.ok(intermediate.titleLineCount > 1, STUDENT_LAYOUT_SENTENCE);
    assert.ok(intermediate.titleRight <= intermediate.viewportWidth + 1, STUDENT_LAYOUT_SENTENCE);
    assert.ok(
      Math.abs(intermediate.primaryLeft - intermediate.secondaryLeft) < 2,
      STUDENT_LAYOUT_SENTENCE,
    );
    assert.equal(intermediate.primaryText, "Start Weekly Assignment", STUDENT_LAYOUT_SENTENCE);
    assert.equal(intermediate.secondaryText, "Review previous Attempt", STUDENT_LAYOUT_SENTENCE);
    assert.ok(intermediate.primaryRight <= intermediate.viewportWidth + 1, STUDENT_LAYOUT_SENTENCE);
    assert.ok(
      intermediate.secondaryRight <= intermediate.viewportWidth + 1,
      STUDENT_LAYOUT_SENTENCE,
    );
    assert.ok(intermediate.secondaryTop > intermediate.primaryTop, STUDENT_LAYOUT_SENTENCE);
  } finally {
    await browser.close();
  }
});

test("Coursework start facts group questions, deadlines, and Attempt rules", async () => {
  const { StudentAssessmentStartFacts } = await loadDecisionDetailsForSsr();
  const summaryHtml = renderToString(() =>
    createComponent(StudentAssessmentStartFacts, {
      questionCount: 4,
      pointsPossible: 12,
      timeLimitSeconds: 900,
      decision: startSummaryDecision(),
    }),
  );
  const withoutDecisionHtml = renderToString(() =>
    createComponent(StudentAssessmentStartFacts, {
      questionCount: 4,
      pointsPossible: 12,
      timeLimitSeconds: 900,
    }),
  );

  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    await page.setContent(startSummaryDocument(summaryHtml), { waitUntil: "load" });
    const grouped = await page.evaluate(() => {
      function sectionFacts(section) {
        return {
          heading: section.querySelector("h3")?.textContent ?? "",
          labels: [...section.querySelectorAll(":scope > .assessment-facts > div > dt")].map(
            (node) => node.textContent,
          ),
        };
      }
      const root = document.querySelector(".student-assessment-start-facts");
      const decision = root.querySelector(".student-assessment-decision");
      const details = decision.querySelector("details");
      const sections = decision.querySelector(":scope > .student-assessment-decision__sections");
      const probe = document.createElement("div");
      probe.style.width = "var(--ple-space-3)";
      document.body.append(probe);
      const expectedGap = getComputedStyle(probe).width;
      probe.remove();
      const heading = sections.querySelector("h3");
      return {
        workLabels: [
          ...root.querySelectorAll(".student-assessment-start-facts__work > div > dt"),
        ].map((node) => node.textContent),
        detailsOpen: details.open,
        visibleSections: [...sections.querySelectorAll(":scope > section")].map(sectionFacts),
        disclosedSections: [
          ...details.querySelectorAll(".student-assessment-decision__section"),
        ].map(sectionFacts),
        rowGap: getComputedStyle(sections).rowGap,
        expectedGap,
        headingMarginTop: getComputedStyle(heading).marginTop,
        headingMarginBottom: getComputedStyle(heading).marginBottom,
      };
    });

    assert.deepEqual(grouped.workLabels, ["Questions", "Points possible"]);
    assert.equal(grouped.detailsOpen, false);
    assert.deepEqual(grouped.visibleSections, [
      { heading: "Deadlines", labels: ["Due"] },
      { heading: "Attempt rules", labels: ["Time limit", "Attempt limit"] },
    ]);
    assert.deepEqual(grouped.disclosedSections, [
      { heading: "Availability", labels: ["Available", "Closes"] },
      { heading: "Attempt rules", labels: ["Late work"] },
    ]);
    assert.equal(grouped.rowGap, grouped.expectedGap);
    assert.notEqual(grouped.expectedGap, "0px");
    assert.equal(grouped.headingMarginTop, "0px");
    assert.equal(grouped.headingMarginBottom, "0px");

    await page.setContent(startSummaryDocument(withoutDecisionHtml), { waitUntil: "load" });
    const plain = await page.evaluate(() => {
      const root = document.querySelector(".student-assessment-start-facts");
      return {
        workLabels: [
          ...root.querySelectorAll(".student-assessment-start-facts__work > div > dt"),
        ].map((node) => node.textContent),
        sections: [...root.querySelectorAll(".student-assessment-decision__section")].map(
          (section) => ({
            heading: section.querySelector("h3")?.textContent ?? "",
            labels: [...section.querySelectorAll("dt")].map((node) => node.textContent),
          }),
        ),
      };
    });
    assert.deepEqual(plain.workLabels, ["Questions", "Points possible"]);
    assert.deepEqual(plain.sections, [{ heading: "Attempt rules", labels: ["Time limit"] }]);
  } finally {
    await browser.close();
  }
});

test("Question Backend-rendered content may use its own fonts when needed for correct display", async () => {
  const baseline = readFileSync(new URL("../../src/styles/ple_embed.css", import.meta.url), "utf8");
  const document = `<!doctype html>
<html>
<head>
<style>
mjx-container { font-family: "MathJax_Main", serif; }
</style>
<style>${baseline.replaceAll("</style", "<\\/style")}</style>
<style>
.question-authored-label { font-family: "STIX Two Text", serif; }
.problem-main-form .problem-content .special-widget { font-family: "Question Widget", monospace; }
</style>
</head>
<body>
<p id="ordinary">Ordinary backend text</p>
<p id="authored" class="question-authored-label">Authored label</p>
<mjx-container id="math">x</mjx-container>
<code id="code">code</code>
<form class="problem-main-form"><div class="problem-content">
<input id="plain" type="text" value="plain">
<input id="widget" class="special-widget" type="text" value="widget">
</div></form>
</body>
</html>`;
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage();
    await page.setContent(document, { waitUntil: "load" });
    const fonts = await page.evaluate(() => {
      const firstFamily = (id) =>
        getComputedStyle(document.getElementById(id))
          .fontFamily.split(",")[0]
          .trim()
          .replaceAll('"', "");
      return {
        ordinary: firstFamily("ordinary"),
        authored: firstFamily("authored"),
        math: firstFamily("math"),
        code: firstFamily("code"),
        plain: firstFamily("plain"),
        widget: firstFamily("widget"),
      };
    });
    assert.equal(fonts.ordinary, "Atkinson Hyperlegible Next");
    assert.equal(fonts.authored, "STIX Two Text");
    assert.equal(fonts.math, "MathJax_Main");
    assert.equal(fonts.code, "Atkinson Hyperlegible Mono");
    assert.equal(fonts.plain, "Atkinson Hyperlegible Next");
    assert.equal(fonts.widget, "Question Widget");
  } finally {
    await browser.close();
  }
});

async function loadMatchingLayoutBundle() {
  const result = await build({
    bundle: true,
    format: "iife",
    globalName: "ResponseLayout",
    outfile: "response_layout.js",
    platform: "browser",
    write: false,
    plugins: [solidPlugin({ solid: { generate: "dom", hydratable: false } })],
    stdin: {
      contents: `
        import { createComponent } from "solid-js";
        import { render } from "solid-js/web";
        import { QuestionResponseControl } from "./question_response_control.tsx";

        const responseFormat = {
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
        };

        export function mountMatching(target) {
          const saves = [];
          window.responseSaves = saves;
          render(
            () =>
              createComponent(QuestionResponseControl, {
                attemptId: "attempt-layout",
                mode: "save",
                responseFormat,
                saveLabel: "Save response",
                validator: { validateResponseFormat: async () => ({ issues: [] }) },
                onEscape: () => undefined,
                onSave: async (response) => {
                  saves.push(JSON.parse(JSON.stringify(response)));
                  return { kind: "accepted" };
                },
              }),
            target,
          );
        }
      `,
      resolveDir: new URL("../../src/components/question_response_controls/", import.meta.url)
        .pathname,
      sourcefile: "response_layout_harness.tsx",
      loader: "tsx",
    },
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) throw new Error("Matching layout bundle is missing JavaScript.");
  return Buffer.from(javascript.contents).toString("utf8");
}

function measureMatchingLayout() {
  const bank = document.querySelector(".matching-bank").getBoundingClientRect();
  const prompts = document.querySelector(".matching-prompts").getBoundingClientRect();
  const columns = getComputedStyle(document.querySelector(".matching-layout"))
    .gridTemplateColumns.trim()
    .split(/\s+/u).length;
  const verticalOverlap = Math.min(bank.bottom, prompts.bottom) - Math.max(bank.top, prompts.top);
  const horizontalOverlap = Math.min(bank.right, prompts.right) - Math.max(bank.left, prompts.left);
  return {
    columns,
    promptText: [...document.querySelectorAll(".matching-prompt")].map((node) => node.textContent),
    choiceIds: [...document.querySelectorAll(".matching-bank [data-choice-id]")].map((node) =>
      node.getAttribute("data-choice-id"),
    ),
    slotText: [...document.querySelectorAll(".matching-slot")].map((node) => node.textContent),
    styleHasAdaptiveBank: [...document.querySelectorAll("style")].some((node) =>
      node.textContent.includes(
        ".question-response-control .matching-layout { display: grid; grid-template-columns: repeat(auto-fit, minmax(min(100%, 18rem), 1fr));",
      ),
    ),
    sideBySide: bank.right <= prompts.left + 1 && verticalOverlap > 0,
    stacked: prompts.top >= bank.bottom - 1 && horizontalOverlap > 0,
  };
}

test("Question response layouts may adapt to available screen space while preserving the same content, response meaning, and grading behavior", async () => {
  const bundle = await loadMatchingLayoutBundle();
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    await page.setContent(
      '<!doctype html><html><head><style>body { margin: 0; }</style></head><body><div id="response-layout"></div></body></html>',
      { waitUntil: "load" },
    );
    await page.addScriptTag({ content: bundle });
    await page.evaluate(() => {
      window.ResponseLayout.mountMatching(document.getElementById("response-layout"));
    });
    const wide = await page.evaluate(measureMatchingLayout);
    assert.equal(wide.styleHasAdaptiveBank, true);
    assert.deepEqual(wide.promptText, ["Mitochondria", "Ribosome"]);
    assert.deepEqual(wide.choiceIds, ["choice-energy", "choice-protein"]);
    assert.equal(wide.sideBySide, true);
    assert.equal(wide.stacked, false);
    assert.ok(wide.columns >= 2);

    await page.getByRole("button", { name: /Makes energy/ }).click();
    await page.getByRole("button", { name: /Mitochondria: Unanswered/ }).click();
    await page.getByRole("button", { name: "Save response", exact: true }).click();
    await page.waitForFunction(() => window.responseSaves.length === 1);
    const wideSave = await page.evaluate(() => window.responseSaves[0]);

    await page.setViewportSize({ width: 360, height: 800 });
    const narrow = await page.evaluate(measureMatchingLayout);
    assert.equal(narrow.styleHasAdaptiveBank, true);
    assert.deepEqual(narrow.promptText, ["Mitochondria", "Ribosome"]);
    assert.deepEqual(narrow.choiceIds, ["choice-energy", "choice-protein"]);
    assert.equal(narrow.stacked, true);
    assert.equal(narrow.sideBySide, false);
    assert.equal(narrow.columns, 1);
    assert.ok(narrow.slotText.some((text) => text.includes("Makes energy")));

    await page.getByRole("button", { name: "Save response", exact: true }).click();
    await page.waitForFunction(() => window.responseSaves.length === 2);
    const narrowSave = await page.evaluate(() => window.responseSaves[1]);
    const savedMatches = {
      kind: "matching",
      matches: [{ prompt: "prompt-mito", choice: "choice-energy" }],
    };
    assert.deepEqual(wideSave, savedMatches);
    assert.deepEqual(narrowSave, wideSave);
  } finally {
    await browser.close();
  }
});

async function openMatchingPage(context, bundle) {
  const page = await context.newPage();
  await page.setContent(
    '<!doctype html><html><head><style>body { margin: 0; }</style></head><body><div id="response-layout"></div></body></html>',
    { waitUntil: "load" },
  );
  await page.addScriptTag({ content: bundle });
  await page.evaluate(() => {
    window.ResponseLayout.mountMatching(document.getElementById("response-layout"));
  });
  await page.locator(".matching-bank").waitFor();
  return page;
}

async function matchingBank(page) {
  return page.evaluate(() => {
    const bank = document.querySelector(".matching-bank");
    const box = bank.getBoundingClientRect();
    return {
      visible: box.width > 0 && box.height > 0,
      choices: [...bank.querySelectorAll("[data-choice-id]")].map((node) => ({
        id: node.getAttribute("data-choice-id"),
        disabled: node.disabled,
        label: node.textContent,
      })),
    };
  });
}

function assertBankReachable(bank, enabledIds) {
  assert.equal(bank.visible, true);
  assert.deepEqual(
    bank.choices.map((choice) => choice.id),
    ["choice-energy", "choice-protein"],
  );
  assert.deepEqual(
    bank.choices.filter((choice) => choice.disabled === false).map((choice) => choice.id),
    enabledIds,
  );
}

async function slotText(page, promptId) {
  return page.locator(`[data-prompt-id="${promptId}"]`).innerText();
}

async function tapControl(page, selector) {
  const box = await page.locator(selector).boundingBox();
  if (box === null) throw new Error(`Missing matching control ${selector}`);
  await page.touchscreen.tap(box.x + box.width / 2, box.y + box.height / 2);
}

test("MATCH Questions should make each prompt's assigned choice easy to recognize and keep the choice bank reachable while Students assign, change, and clear matches using keyboard, pointer, or touch", async () => {
  const bundle = await loadMatchingLayoutBundle();
  const browser = await chromium.launch({ headless: true });
  try {
    const context = await browser.newContext({
      viewport: { width: 1100, height: 800 },
      hasTouch: true,
    });
    try {
      const pointer = await openMatchingPage(context, bundle);
      await pointer.locator('[data-choice-id="choice-energy"]').click();
      await pointer.locator('[data-prompt-id="prompt-mito"]').click();
      assert.equal(await slotText(pointer, "prompt-mito"), "Makes energy");
      const pointerAssigned = await matchingBank(pointer);
      assertBankReachable(pointerAssigned, ["choice-protein"]);
      assert.match(
        pointerAssigned.choices.find((choice) => choice.id === "choice-energy").label,
        /Makes energy/,
      );
      assert.match(
        pointerAssigned.choices.find((choice) => choice.id === "choice-energy").label,
        /Used in 1 slot/,
      );
      await pointer.locator('[data-choice-id="choice-protein"]').click();
      await pointer.locator('[data-prompt-id="prompt-mito"]').click();
      assert.equal(await slotText(pointer, "prompt-mito"), "Builds proteins");
      assertBankReachable(await matchingBank(pointer), ["choice-energy"]);
      await pointer
        .getByRole("button", { name: "Clear response for Mitochondria", exact: true })
        .click();
      assert.equal(await slotText(pointer, "prompt-mito"), "Assign selected choice");
      assertBankReachable(await matchingBank(pointer), ["choice-energy", "choice-protein"]);

      const keyboard = await openMatchingPage(context, bundle);
      await keyboard.keyboard.press("Tab");
      assert.equal(
        await keyboard.evaluate(() => document.activeElement?.dataset.choiceId),
        "choice-energy",
      );
      await keyboard.keyboard.press("Space");
      await keyboard
        .getByText("Selected: Makes energy. Activate a prompt slot to assign it.")
        .waitFor();
      await keyboard.keyboard.press("Tab");
      await keyboard.keyboard.press("Tab");
      assert.equal(
        await keyboard.evaluate(() => document.activeElement?.dataset.promptId),
        "prompt-mito",
      );
      await keyboard.keyboard.press("Enter");
      assert.equal(await slotText(keyboard, "prompt-mito"), "Makes energy");
      assertBankReachable(await matchingBank(keyboard), ["choice-protein"]);
      await keyboard.keyboard.press("Shift+Tab");
      assert.equal(
        await keyboard.evaluate(() => document.activeElement?.dataset.choiceId),
        "choice-protein",
      );
      await keyboard.keyboard.press("Enter");
      await keyboard.keyboard.press("Tab");
      assert.equal(
        await keyboard.evaluate(() => document.activeElement?.dataset.promptId),
        "prompt-mito",
      );
      await keyboard.keyboard.press("Enter");
      assert.equal(await slotText(keyboard, "prompt-mito"), "Builds proteins");
      assertBankReachable(await matchingBank(keyboard), ["choice-energy"]);
      await keyboard.keyboard.press("Tab");
      assert.match(
        await keyboard.evaluate(() => document.activeElement?.getAttribute("aria-label") ?? ""),
        /^Clear response for Mitochondria/,
      );
      await keyboard.keyboard.press("Enter");
      assert.equal(await slotText(keyboard, "prompt-mito"), "Assign selected choice");
      assertBankReachable(await matchingBank(keyboard), ["choice-energy", "choice-protein"]);

      const touch = await openMatchingPage(context, bundle);
      await tapControl(touch, '[data-choice-id="choice-energy"]');
      await tapControl(touch, '[data-prompt-id="prompt-mito"]');
      assert.equal(await slotText(touch, "prompt-mito"), "Makes energy");
      assertBankReachable(await matchingBank(touch), ["choice-protein"]);
      await tapControl(touch, '[data-choice-id="choice-protein"]');
      await tapControl(touch, '[data-prompt-id="prompt-mito"]');
      assert.equal(await slotText(touch, "prompt-mito"), "Builds proteins");
      assertBankReachable(await matchingBank(touch), ["choice-energy"]);
      await tapControl(touch, 'button[aria-label="Clear response for Mitochondria"]');
      assert.equal(await slotText(touch, "prompt-mito"), "Assign selected choice");
      assertBankReachable(await matchingBank(touch), ["choice-energy", "choice-protein"]);
    } finally {
      await context.close();
    }
  } finally {
    await browser.close();
  }
});
