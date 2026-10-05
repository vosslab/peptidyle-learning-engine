import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";

import {
  appendPickedFixedEntries,
  appendPickedPool,
  blueprintLifecyclePresentation,
  emptyReusableContent,
  moveReusableEntry,
  reusableContentInputFromView,
  validateBlueprintCourseContent,
  validateReusableContent,
} from "../src/features/blueprint_course/blueprint_course_model.ts";
import {
  assessmentComparison,
  publishedQuestionComparison,
  questionPoolComparison,
} from "../src/features/blueprint_forks/blueprint_fork_model.ts";

function selection(...questionIds) {
  return {
    questionIds,
    questions: questionIds.map((questionId) => ({
      questionId,
      row: {
        publishedQuestionRevisionTuple: { publishedQuestionId: questionId, revisionNumber: 1 },
      },
    })),
  };
}

test("new Blueprint Assessment working state keeps answer-bearing feedback private", () => {
  const feedback =
    emptyReusableContent("regular_assignment").defaults.student_feedback_release_rule;

  assert.deepEqual(feedback, {
    per_item_correctness: "after_submit",
    submitted_response: "after_submit",
    question_answer: "never",
    question_answer_explanation: "never",
    class_statistics: "never",
    hints: "never",
    worked_solutions: "never",
  });
});

test("Blueprint lifecycle choices expose only the server-directed state transitions", () => {
  const privateOwner = blueprintLifecyclePresentation("private", "blueprint_course_owner");
  assert.equal(privateOwner.canEdit, true);
  assert.equal(privateOwner.canPublish, true);
  assert.equal(privateOwner.canArchive, false);
  assert.equal(privateOwner.canAdopt, false);

  const publicReader = blueprintLifecyclePresentation("public", "active_instructor");
  assert.equal(publicReader.canAdopt, true);
  assert.equal(publicReader.canEdit, false);
  assert.equal(publicReader.canArchive, false);

  const publicOwner = blueprintLifecyclePresentation("public", "blueprint_course_owner");
  assert.equal(publicOwner.canEdit, true);
  assert.equal(publicOwner.canArchive, true);
  assert.equal(publicOwner.canReturnToPrivate, true);

  const archivedOwner = blueprintLifecyclePresentation("archived", "blueprint_course_owner");
  assert.equal(archivedOwner.canRestore, true);
  assert.equal(archivedOwner.canAdopt, false);
  assert.equal(archivedOwner.canEdit, false);

  const archivedReader = blueprintLifecyclePresentation("archived", "active_instructor");
  assert.equal(archivedReader.canEdit, false);
});

test("new Blueprint Assessment working state uses the Assessment delivery defaults", () => {
  const defaults = emptyReusableContent("quiz", "Quiz").defaults;

  assert.equal(defaults.late_work_rule, "reject");
  assert.equal(defaults.assessment_attempt_limit, 1);
  assert.equal(defaults.student_feedback_release_rule.question_answer, "after_submit");
  assert.equal(defaults.student_feedback_release_rule.question_answer_explanation, "after_submit");
  assert.equal(defaults.activity_rules.assessmentQuestionOrderRule, "shuffled");
});

test("new regular Blueprint Assessment remains unlimited without a score or correctness gate", () => {
  const defaults = emptyReusableContent("regular_assignment").defaults;

  assert.equal(defaults.assessment_attempt_limit, null);
});

test("reusable entries preserve fixed and Question Pool interleaving", () => {
  const fixed = appendPickedFixedEntries(
    emptyReusableContent("quiz", "Quiz"),
    selection("AAAA-2BBB"),
  );
  const pooled = appendPickedPool(fixed, "CCCD-NDDD");
  const reordered = moveReusableEntry(pooled, 1, -1);

  assert.deepEqual(
    reordered.entries.map((entry) => entry.kind),
    ["pool", "fixed"],
  );
  assert.equal(reordered.entries[0]?.kind === "pool" && reordered.entries[0].selection_count, 1);
});

test("Question Pool validation requires a positive whole selection count", () => {
  const content = appendPickedPool(emptyReusableContent("quiz", "Quiz"), "AAAA-2BBB");
  const pool = content.entries[0];
  const invalid =
    pool?.kind === "pool" ? { ...content, entries: [{ ...pool, selection_count: 0 }] } : content;

  assert.match(validateReusableContent(invalid).message ?? "", /selection count/);
});

test("Blueprint Course creation requires separate short and long lineage names", () => {
  const assignment = {
    ...emptyReusableContent("regular_assignment", "Ready assignment"),
    entries: [
      {
        kind: "fixed",
        published_question_revision_tuple: { publishedQuestionId: "AAAA-2BBB", revisionNumber: 1 },
        points_possible: "1",
        scoring_rule: "normal",
      },
    ],
  };
  assert.equal(
    validateBlueprintCourseContent({
      short_name: "Blueprint",
      classification: {
        disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
        subjectUuid: null,
        topicUuid: null,
        subtopicUuid: null,
        tags: [],
      },
      long_name: "Protein folding Blueprint Course",
      modules: [{ label: "Module 1", assessments: [assignment] }],
    }).valid,
    true,
  );
  assert.match(
    validateBlueprintCourseContent({
      short_name: " ",
      classification: {
        disciplineUuid: "018f5e7d-01b6-7c14-8a0b-4bfef6390d6d",
        subjectUuid: null,
        topicUuid: null,
        subtopicUuid: null,
        tags: [],
      },
      long_name: "Protein folding Blueprint Course",
      modules: [{ label: "Module 1", assessments: [assignment] }],
    }).message ?? "",
    /short and long names/,
  );
});

test("saved Blueprint Assessment mapping preserves its selected Type", () => {
  const defaults = emptyReusableContent("regular_assignment").defaults;
  const mapped = reusableContentInputFromView({
    assessment_type: "exam",
    title: "Final exam",
    instructions: "Show your reasoning.",
    entries: [],
    defaults,
  });

  assert.equal(mapped.assessment_type, "exam");
  assert.deepEqual(mapped.defaults, defaults);
});

function fixedQuestion(id, revision) {
  return {
    kind: "fixed",
    published_question_revision_tuple: { publishedQuestionId: id, revisionNumber: revision },
    points_possible: "1",
    scoring_rule: "normal",
    question_attempt_limit: { maxAttempts: null },
    question_attempt_time_limit: { kind: "unlimited" },
  };
}

function questionPool(id, edit) {
  return {
    kind: "pool",
    question_pool_id: id,
    question_pool_edit_number: edit,
    selection_count: 1,
    points_per_item: "1",
    scoring_rule: "normal",
    selection_rule: { selectedQuestionOrder: "questionPoolOrder" },
    question_attempt_limit: { maxAttempts: null },
    question_attempt_time_limit: { kind: "unlimited" },
  };
}

function comparedAssessment(id, moduleId, position, title, entries, questionIds) {
  return {
    blueprintAssessmentId: id,
    blueprintModuleId: moduleId,
    position,
    content: {
      assessment_type: "quiz",
      title,
      instructions: "",
      entries,
      defaults: emptyReusableContent("quiz").defaults,
    },
    questionIds,
  };
}

async function loadComparisonMembershipSummary() {
  const result = await build({
    bundle: true,
    entryPoints: [
      new URL(
        "../src/features/blueprint_forks/blueprint_comparison_membership.tsx",
        import.meta.url,
      ).pathname,
    ],
    format: "esm",
    outfile: "blueprint_comparison_membership.js",
    platform: "node",
    plugins: [solidPlugin({ solid: { generate: "ssr", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined)
    throw new Error("comparison membership bundle is missing JavaScript.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module.ComparisonMembershipSummary !== "function") {
    throw new Error("comparison membership bundle does not export ComparisonMembershipSummary.");
  }
  return module.ComparisonMembershipSummary;
}

test("comparison shows shared, added, removed, and changed Assessments, Published Questions, and Question Pools", async () => {
  const left = {
    modules: [{ blueprintModuleId: "left-module", label: "Week 1", position: 0 }],
    assessments: [
      comparedAssessment(
        "left-stable",
        "left-module",
        0,
        "Stable lab",
        [fixedQuestion("STABLE-Q", 1), questionPool("STABLE-POOL", 1)],
        ["STABLE-Q"],
      ),
      comparedAssessment(
        "left-shared",
        "left-module",
        1,
        "Shared quiz",
        [
          fixedQuestion("CHANGED-Q", 1),
          questionPool("CHANGED-POOL", 1),
          questionPool("REMOVED-POOL", 1),
        ],
        ["CHANGED-Q"],
      ),
      comparedAssessment(
        "left-removed",
        "left-module",
        2,
        "Removed quiz",
        [fixedQuestion("REMOVED-Q", 1)],
        ["REMOVED-Q"],
      ),
    ],
  };
  const right = {
    modules: [{ blueprintModuleId: "right-module", label: "Week 1", position: 0 }],
    assessments: [
      comparedAssessment(
        "right-stable",
        "right-module",
        0,
        "Stable lab",
        [fixedQuestion("STABLE-Q", 1), questionPool("STABLE-POOL", 1)],
        ["STABLE-Q"],
      ),
      comparedAssessment(
        "right-shared",
        "right-module",
        1,
        "Shared quiz",
        [
          fixedQuestion("CHANGED-Q", 2),
          questionPool("CHANGED-POOL", 2),
          questionPool("ADDED-POOL", 1),
        ],
        ["CHANGED-Q"],
      ),
      comparedAssessment(
        "right-added",
        "right-module",
        2,
        "Added quiz",
        [fixedQuestion("ADDED-Q", 1)],
        ["ADDED-Q"],
      ),
    ],
  };
  const view = {
    left,
    right,
    assessmentRelationships: [
      {
        leftAssessmentId: "left-stable",
        rightAssessmentId: "right-stable",
        sharedQuestionIds: ["STABLE-Q"],
      },
      {
        leftAssessmentId: "left-shared",
        rightAssessmentId: "right-shared",
        sharedQuestionIds: ["CHANGED-Q"],
      },
    ],
  };

  assert.deepEqual(publishedQuestionComparison(view), {
    shared: ["CHANGED-Q", "STABLE-Q"],
    added: ["ADDED-Q"],
    removed: ["REMOVED-Q"],
    changed: ["CHANGED-Q"],
  });
  assert.deepEqual(questionPoolComparison(view), {
    shared: ["STABLE-POOL"],
    added: ["ADDED-POOL"],
    removed: ["REMOVED-POOL"],
    changed: ["CHANGED-POOL"],
  });
  const assessments = assessmentComparison(view);
  assert.deepEqual(assessments.shared, ["Stable lab compared with Stable lab"]);
  assert.deepEqual(assessments.added, ["Added quiz"]);
  assert.deepEqual(assessments.removed, ["Removed quiz"]);
  assert.deepEqual(assessments.changed, [
    "Shared quiz compared with Shared quiz: Current differences: Questions, Pools, pins, order or scoring.",
  ]);

  const ComparisonMembershipSummary = await loadComparisonMembershipSummary();
  const html = renderToString(() => createComponent(ComparisonMembershipSummary, { view }));
  assert.match(html, /Shared Assessments: Stable lab compared with Stable lab/);
  assert.match(html, /Added Assessments: Added quiz/);
  assert.match(html, /Removed Assessments: Removed quiz/);
  assert.match(html, /Changed Assessments: Shared quiz compared with Shared quiz/);
  assert.match(html, /Questions, Pools, pins, order or scoring/);
  assert.match(html, /Shared Published Questions: CHANGED-Q, STABLE-Q/);
  assert.match(html, /Added Published Questions: ADDED-Q/);
  assert.match(html, /Removed Published Questions: REMOVED-Q/);
  assert.match(html, /Changed Published Questions: CHANGED-Q/);
  assert.match(html, /Shared Question Pools: STABLE-POOL/);
  assert.match(html, /Added Question Pools: ADDED-POOL/);
  assert.match(html, /Removed Question Pools: REMOVED-POOL/);
  assert.match(html, /Changed Question Pools: CHANGED-POOL/);

  const emptyHtml = renderToString(() =>
    createComponent(ComparisonMembershipSummary, {
      view: {
        left: { modules: [], assessments: [] },
        right: { modules: [], assessments: [] },
        assessmentRelationships: [],
      },
    }),
  );
  for (const label of [
    "Shared Assessments",
    "Added Assessments",
    "Removed Assessments",
    "Changed Assessments",
    "Shared Published Questions",
    "Added Published Questions",
    "Removed Published Questions",
    "Changed Published Questions",
    "Shared Question Pools",
    "Added Question Pools",
    "Removed Question Pools",
    "Changed Question Pools",
  ]) {
    assert.match(emptyHtml, new RegExp(`${label}: None`));
  }
});
