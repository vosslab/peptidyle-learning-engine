// Pure route, session-state, and issued-question contract evidence.

import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";
import { createComponent } from "solid-js";
import { renderToString } from "solid-js/web";

import { createHttpApiClient } from "../src/api/http_client.ts";
import { createBrowserSessionBoundary } from "../src/auth/browser_session_boundary.ts";
import { createSessionBootstrap, sessionFailureState } from "../src/auth/session_context.tsx";
import {
  createDisciplineFromRequest,
  submitDisciplineRequest,
} from "../src/components/discipline_request.ts";
import { submitSysadminCourseCreation } from "../src/components/sysadmin_course_creation.ts";
import { userRoleMayAccessRoute, routeContractForPathname } from "../src/route_contract.ts";

test("route contracts fail closed and reserve declared teaching routes for instructors", () => {
  assert.equal(routeContractForPathname("/library/7K3M-79QP")?.id, "questionDetail");
  assert.equal(routeContractForPathname("/library/7K3M-79QP/extra"), undefined);
  assert.equal(routeContractForPathname("/blueprint-courses")?.id, "blueprintCourses");
  assert.equal(
    routeContractForPathname("/blueprint-courses/search/public")?.id,
    "publicBlueprintSearch",
  );
  assert.equal(userRoleMayAccessRoute("publicBlueprintSearch", "instructor"), true);
  assert.equal(userRoleMayAccessRoute("publicBlueprintSearch", "student"), false);
  assert.equal(userRoleMayAccessRoute("publicBlueprintSearch", "sysadmin"), false);
  assert.equal(routeContractForPathname("/blueprint-courses/BP-7")?.id, "blueprintCourseDetail");
  assert.equal(routeContractForPathname("/blueprint-courses/BP-7/extra"), undefined);
  assert.equal(routeContractForPathname("/sysadmin/instructor-approval"), undefined);
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF")?.id,
    "assessmentWorkspaceOverview",
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/questions")?.id,
    "assessmentWorkspaceQuestions",
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/properties")?.id,
    "assessmentWorkspacePolicies",
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/student-view")
      ?.id,
    "assessmentWorkspaceStudentView",
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/delivery-check"),
    undefined,
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/release"),
    undefined,
  );
  assert.equal(
    routeContractForPathname(
      "/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/presentations/0123456789abcdef0123456789abcdef",
    ),
    undefined,
  );
  assert.equal(
    routeContractForPathname("/instructor/courses/CI7K3M2QAZ/assessments/A9D2RX5AF/edit"),
    undefined,
  );
  assert.equal(userRoleMayAccessRoute("assessmentOverview", "student"), true);
  assert.equal(userRoleMayAccessRoute("assessmentOverview", "instructor"), false);
  assert.equal(userRoleMayAccessRoute("courseAssessments", "student"), false);
  assert.equal(userRoleMayAccessRoute("courseAssessments", "instructor"), true);
  assert.equal(userRoleMayAccessRoute("assessmentWorkspaceOverview", "student"), false);
  assert.equal(userRoleMayAccessRoute("assessmentWorkspaceOverview", "instructor"), true);
  assert.equal(routeContractForPathname("/workspace"), undefined);
  assert.equal(routeContractForPathname("/workspace/draft-1"), undefined);
  assert.equal(routeContractForPathname("/instructor/courses/C-1/grade-settings"), undefined);
  assert.equal(routeContractForPathname("/instructor/courses/C-1/teaching-operations"), undefined);
  assert.equal(userRoleMayAccessRoute("blueprintCourses", "student"), false);
  assert.equal(userRoleMayAccessRoute("blueprintCourses", "sysadmin"), false);
  assert.equal(userRoleMayAccessRoute("blueprintCourses", "instructor"), true);
  assert.equal(routeContractForPathname("/account-settings"), undefined);
  assert.equal(routeContractForPathname("/account-settings/credentials"), undefined);
});

test("session bootstrap retains only safe session state with direct narrow dependencies", async () => {
  const session = {
    authenticated: true,
    account: { id: "account-a", role: "student" },
  };
  const boundaryStates = [];
  let bootstrap;
  bootstrap = createSessionBootstrap(
    async () => session,
    async () => undefined,
    () => boundaryStates.push(bootstrap.state().kind),
  );
  await bootstrap.retry();
  assert.equal(bootstrap.state().kind, "authenticated");
  assert.equal("credential" in bootstrap.state(), false);
  assert.equal(await bootstrap.signOut(), true);
  await bootstrap.retry();
  assert.deepEqual(boundaryStates, ["authenticated", "loading"]);
  assert.deepEqual(sessionFailureState({ status: 401 }), { kind: "expired" });
});

test("browser session generations abort old requests and clear cached projections", async () => {
  const signals = [];
  let cacheClears = 0;
  const boundary = createBrowserSessionBoundary(
    async (_input, init) => {
      signals.push(init?.signal);
      return new Response(null, { status: 204 });
    },
    () => {
      cacheClears += 1;
    },
  );

  await boundary.fetch("/first");
  const firstSignal = signals.at(-1);
  assert.equal(firstSignal?.aborted, false);
  boundary.advance();
  assert.equal(firstSignal?.aborted, true);
  assert.equal(cacheClears, 1);

  await boundary.fetch("/second");
  const secondSignal = signals.at(-1);
  assert.notEqual(secondSignal, firstSignal);
  assert.equal(secondSignal?.aborted, false);

  const request = new AbortController();
  await boundary.fetch("/third", { signal: request.signal });
  const combinedSignal = signals.at(-1);
  request.abort();
  assert.equal(combinedSignal?.aborted, true);
  assert.equal(secondSignal?.aborted, false);
});

test("a stale session lookup cannot overwrite a newer authenticated generation", async () => {
  let releaseFirst;
  let calls = 0;
  const firstSession = new Promise((resolve) => {
    releaseFirst = resolve;
  });
  const newerSession = {
    authenticated: true,
    account: { id: "account-new", role: "student" },
  };
  let advances = 0;
  const bootstrap = createSessionBootstrap(
    async () => {
      calls += 1;
      return calls === 1 ? firstSession : newerSession;
    },
    async () => undefined,
    () => {
      advances += 1;
    },
  );

  const stale = bootstrap.retry();
  await bootstrap.retry();
  assert.deepEqual(bootstrap.state(), { kind: "authenticated", session: newerSession });
  releaseFirst({
    authenticated: true,
    account: { id: "account-old", role: "student" },
  });
  await stale;
  assert.deepEqual(bootstrap.state(), { kind: "authenticated", session: newerSession });
  assert.equal(advances, 1);
});

async function loadProfileAccountId() {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("../src/pages/profile_account_id.tsx", import.meta.url).pathname],
    format: "esm",
    outfile: "profile_account_id.js",
    platform: "node",
    plugins: [solidPlugin({ solid: { generate: "ssr", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) throw new Error("Profile Account ID bundle is missing JavaScript.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module.ProfileAccountId !== "function") {
    throw new Error("Profile Account ID bundle does not export the component.");
  }
  return module.ProfileAccountId;
}

async function loadCompactShortNameField() {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("../src/pages/compact_short_name_field.tsx", import.meta.url).pathname],
    format: "esm",
    outfile: "compact_short_name_field.js",
    platform: "node",
    plugins: [solidPlugin({ solid: { generate: "ssr", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) throw new Error("Short name field bundle is missing JavaScript.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module.CompactShortNameField !== "function") {
    throw new Error("Short name field bundle does not export the component.");
  }
  return module.CompactShortNameField;
}

test("the Blueprint short name field states the compact-navigation guidance", async () => {
  const page = fs.readFileSync(
    new URL("../src/pages/course_instance_page.tsx", import.meta.url),
    "utf8",
  );
  assert.match(page, /<CompactShortNameField/);
  assert.match(page, /helpId="course-instance-blueprint-short-name-help"/);
  for (const relative of [
    "../src/pages/course_list_page.tsx",
    "../src/features/blueprint_course/blueprint_course_create_dialog.tsx",
    "../src/features/blueprint_course/blueprint_course_detail_structure.tsx",
    "../src/components/sysadmin_course_creation_form.tsx",
  ]) {
    const source = fs.readFileSync(new URL(relative, import.meta.url), "utf8");
    assert.match(source, /For compact navigation; about 16 characters when practical\./);
  }
  const CompactShortNameField = await loadCompactShortNameField();
  const html = renderToString(() =>
    createComponent(CompactShortNameField, {
      label: "Blueprint short name",
      name: "shortName",
      value: "Upper-Level Biochemistry",
      maxLength: 200,
      helpId: "course-instance-blueprint-short-name-help",
      onInput: () => undefined,
    }),
  );
  assert.match(html, /Blueprint short name/);
  assert.match(html, /For compact navigation; about 16 characters when practical\./);
  assert.match(html, /value="Upper-Level Biochemistry"/);
  assert.match(html, /maxlength="200"/);
  assert.match(html, /aria-describedby="course-instance-blueprint-short-name-help"/);
  assert.match(html, /id="course-instance-blueprint-short-name-help"/);
});

test("the profile page shows the signed-in Account ID as a labeled fact", async () => {
  const profile = fs.readFileSync(
    new URL("../src/pages/profile_page.tsx", import.meta.url),
    "utf8",
  );
  assert.match(profile, /authenticatedAccountId\(\)/);
  assert.match(profile, /<ProfileAccountId/);
  const ProfileAccountId = await loadProfileAccountId();
  const html = renderToString(() => createComponent(ProfileAccountId, { accountId: "U0000035E" }));
  assert.match(html, />Account ID</);
  assert.match(html, /aria-label="Account ID U0000035E"/);
  assert.match(html, /Copy Account ID U0000035E/);
  const hidden = renderToString(() =>
    createComponent(ProfileAccountId, { accountId: "not-an-account" }),
  );
  assert.equal(hidden, "");
});

async function loadDisciplineRequestControl() {
  const result = await build({
    bundle: true,
    entryPoints: [
      new URL("../src/components/course_classification_fields.tsx", import.meta.url).pathname,
    ],
    format: "esm",
    outfile: "course_classification_fields.js",
    platform: "node",
    plugins: [solidPlugin({ solid: { generate: "ssr", hydratable: false } })],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) throw new Error("Discipline request bundle is missing JavaScript.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module.DisciplineRequestControl !== "function") {
    throw new Error("Discipline request bundle does not export the control.");
  }
  if (typeof module.ApplicationApiProvider !== "function") {
    throw new Error("Discipline request bundle does not export the API provider.");
  }
  return module;
}

test("Course Discipline selection requests a new Discipline without creating it", async () => {
  const fields = fs.readFileSync(
    new URL("../src/components/course_classification_fields.tsx", import.meta.url),
    "utf8",
  );
  const disciplines = fs.readFileSync(
    new URL("../src/pages/content_disciplines_page.tsx", import.meta.url),
    "utf8",
  );
  assert.match(fields, /<DisciplineRequestControl/);
  assert.match(fields, /Request a new Discipline/);
  assert.match(disciplines, /createDisciplineFromRequest/);
  assert.match(disciplines, /Requester Account ID/);
  const calls = [];
  const message = await submitDisciplineRequest(
    {
      requestContentDiscipline: async (name) => {
        calls.push(name);
        return {
          uuid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
          requestedName: name,
          requestedByAccountId: "U0000035E",
        };
      },
    },
    "  Genetics  ",
  );
  assert.deepEqual(calls, ["Genetics"]);
  assert.match(message, /Discipline request recorded for Genetics/);
  assert.match(message, /A Sysadmin creates Disciplines/);
  const fulfilled = [];
  const discipline = await createDisciplineFromRequest(
    {
      fulfillDisciplineRequest: async (uuid) => {
        fulfilled.push(uuid);
        return {
          uuid: "bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
          name: "Genetics",
          isRetired: false,
        };
      },
    },
    {
      uuid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
      requestedName: "Genetics",
      requestedByAccountId: "U0000035E",
    },
  );
  assert.equal(discipline.name, "Genetics");
  assert.deepEqual(fulfilled, ["aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"]);
  await assert.rejects(
    () =>
      submitDisciplineRequest(
        {
          requestContentDiscipline: async () => {
            throw new Error("must not send");
          },
        },
        "   ",
      ),
    /1 through 120 characters/,
  );
});

test("a Sysadmin creates a Course that an Instructor teaches", async () => {
  const home = fs.readFileSync(
    new URL("../src/pages/role_home_pages.tsx", import.meta.url),
    "utf8",
  );
  const instructorCourses = fs.readFileSync(
    new URL("../src/pages/course_list_page.tsx", import.meta.url),
    "utf8",
  );
  assert.match(home, /<SysadminCourseCreation/);
  assert.doesNotMatch(home, /ribbon/);
  assert.doesNotMatch(instructorCourses, /assignedInstructorAccountId/);
  const sent = [];
  const message = await submitSysadminCourseCreation(
    {
      createCourseInstance: async (input) => {
        sent.push(input);
        return {
          courseInstance: {
            shortName: input.shortName,
            lifecycleState: "active",
          },
        };
      },
    },
    {
      assignedInstructorAccountId: "U0000035E",
      classification: {
        disciplineUuid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
        subjectUuid: null,
        topicUuid: null,
        subtopicUuid: null,
        tags: [],
      },
      shortName: "Cell Biology",
      longName: "Cell Biology for Majors",
      startDate: "2026-09-30",
      endDate: "2026-10-01",
    },
    "2026-09-30",
  );
  assert.equal(sent.length, 1);
  assert.equal(sent[0].source.kind, "empty");
  assert.equal(sent[0].assignedInstructorAccountId, "U0000035E");
  assert.equal(sent[0].classification.disciplineUuid, "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee");
  assert.match(message, /Course Cell Biology was created/);
  assert.match(message, /Instructor U0000035E teaches it/);
  const refused = [];
  await assert.rejects(
    () =>
      submitSysadminCourseCreation(
        {
          createCourseInstance: async () => {
            refused.push("called");
            throw new Error("must not create");
          },
        },
        {
          assignedInstructorAccountId: "",
          classification: {
            disciplineUuid: "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            subjectUuid: null,
            topicUuid: null,
            subtopicUuid: null,
            tags: [],
          },
          shortName: "Cell Biology",
          longName: "Cell Biology for Majors",
          startDate: "2026-09-30",
          endDate: "2026-10-01",
        },
        "2026-09-30",
      ),
    /Instructor Account ID/,
  );
  assert.deepEqual(refused, []);
  const form = fs.readFileSync(
    new URL("../src/components/sysadmin_course_creation_form.tsx", import.meta.url),
    "utf8",
  );
  assert.match(form, /Instructor Account ID/);
  assert.match(form, /name="assignedInstructorAccountId"/);
  assert.match(form, /instructor\.accountId/);
  assert.match(form, /For compact navigation; about 16 characters when practical\./);
  assert.match(form, /does not add you as an Instructor/);
  const { DisciplineRequestControl, ApplicationApiProvider } = await loadDisciplineRequestControl();
  const html = renderToString(() =>
    createComponent(ApplicationApiProvider, {
      applicationApi: { client: {}, queries: {} },
      get children() {
        return createComponent(DisciplineRequestControl, {});
      },
    }),
  );
  assert.match(html, /Request a new Discipline/);
  assert.match(html, /Requested Discipline name/);
  assert.match(html, /A Sysadmin creates Disciplines/);
});

async function loadStarSurfaceRenderer() {
  const result = await build({
    bundle: true,
    stdin: {
      contents: `
        import { createComponent, Suspense } from "solid-js";
        import { renderToStringAsync } from "solid-js/web";
        import { ApplicationApiProvider } from "./src/api/application_api.tsx";
        import { QuestionPoolStarControl } from "./src/components/question_pool_star_control.tsx";
        import { QuestionStarControl } from "./src/components/question_star_control.tsx";
        function renderControl(applicationApi, Control, props) {
          return renderToStringAsync(() =>
            createComponent(Suspense, {
              get children() {
                return createComponent(ApplicationApiProvider, {
                  applicationApi,
                  get children() {
                    return createComponent(Control, props);
                  },
                });
              },
            }),
          );
        }
        export function renderStarSurfaces(applicationApi, questionId, poolId) {
          return Promise.all([
            renderControl(applicationApi, QuestionStarControl, { questionId }),
            renderControl(applicationApi, QuestionPoolStarControl, { poolId }),
          ]).then(([questionHtml, poolHtml]) => questionHtml + poolHtml);
        }
      `,
      resolveDir: new URL("..", import.meta.url).pathname,
      sourcefile: "star_surface_ssr.js",
      loader: "js",
    },
    format: "esm",
    outfile: "star_surface_ssr.js",
    platform: "node",
    plugins: [
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
      solidPlugin({ solid: { generate: "ssr", hydratable: false } }),
    ],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) throw new Error("Star surface SSR bundle is missing JavaScript.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module.renderStarSurfaces !== "function") {
    throw new Error("Star surface SSR bundle has no renderer.");
  }
  return module.renderStarSurfaces;
}

test("Instructors can see the Star count and the Instructor Profiles behind Question and Pool Stars.", async () => {
  const detail = fs.readFileSync(
    new URL("../src/pages/question_detail_page.tsx", import.meta.url),
    "utf8",
  );
  const poolDetail = fs.readFileSync(
    new URL("../src/pages/question_pool_detail.tsx", import.meta.url),
    "utf8",
  );
  const library = fs.readFileSync(
    new URL("../src/pages/library_page.tsx", import.meta.url),
    "utf8",
  );
  assert.match(
    detail,
    /state\.session\.account\.userRole === "instructor"[\s\S]*<Show when=\{mayMutateLibrary\(\)\}>[\s\S]*<QuestionStarControl questionId=\{record\(\)\.summary\.questionId\} \/>/,
  );
  assert.match(
    poolDetail,
    /<Show when=\{mayMutateLibrary\(\)\}>[\s\S]*<QuestionPoolStarControl poolId=\{value\(\)\.questionPoolId\} \/>/,
  );
  assert.match(
    library,
    /const mayMutateLibrary = sessionScope\.account\.userRole === "instructor"/,
  );
  assert.doesNotMatch(library, /mayWatchPools=/);

  const requests = [];
  const projection = {
    starCount: 2,
    viewerHasStarred: false,
    starredInstructors: [
      { displayName: "Ada Lopez", accountId: "U00000009" },
      { displayName: "Grace Hopper", accountId: "UABCDEFGM" },
    ],
  };
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(String(input), "https://ple.example"), init);
      const pathname = new URL(request.url).pathname;
      requests.push({ method: request.method, pathname });
      const payload = pathname.startsWith("/api/instructor-profiles/")
        ? { displayName: "Ada Lopez", avatar: { kind: "provided", providedAvatarId: "amber-arch" } }
        : projection;
      return new Response(JSON.stringify(payload), {
        headers: { "cache-control": "no-store", "content-type": "application/json" },
      });
    },
  });
  const renderStarSurfaces = await loadStarSurfaceRenderer();
  const html = await renderStarSurfaces({ client, queries: {} }, "7K3M-79QP", "3S8B-24DZ");
  assert.match(html, /2 Instructors have starred this question\./);
  assert.match(html, /2 Instructors have starred this Question Pool\./);
  assert.match(html, /Ada Lopez/);
  assert.match(html, /Grace Hopper/);
  assert.deepEqual(requests.map((request) => `${request.method} ${request.pathname}`).sort(), [
    "GET /api/instructor-profiles/U00000009",
    "GET /api/instructor-profiles/UABCDEFGM",
    "GET /api/question-pools/3S8B-24DZ/stewardship/star",
    "GET /api/questions/by-id/7K3M-79QP/stewardship/star",
  ]);
});

test("Published Questions and Question Pools can be starred and watched.", async () => {
  const questionId = "7K3M-79QP";
  const poolId = "3S8B-24DZ";
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(String(input), "https://ple.example"), init);
      const body = request.method === "GET" ? null : await request.json();
      const pathname = new URL(request.url).pathname;
      requests.push({ method: request.method, pathname, body });
      const payload = pathname.endsWith("/watch")
        ? { watching: body.watching }
        : {
            starCount: body.starred ? 1 : 0,
            viewerHasStarred: body.starred,
            starredInstructors: [],
          };
      return new Response(JSON.stringify(payload), {
        headers: { "cache-control": "no-store", "content-type": "application/json" },
      });
    },
  });

  assert.deepEqual(await client.setQuestionStar(questionId, true), {
    starCount: 1,
    viewerHasStarred: true,
    starredInstructors: [],
  });
  assert.deepEqual(await client.setQuestionPoolStar(poolId, true), {
    starCount: 1,
    viewerHasStarred: true,
    starredInstructors: [],
  });
  assert.deepEqual(await client.setQuestionWatch(questionId, true), { watching: true });
  assert.deepEqual(await client.setQuestionPoolWatch(poolId, true), { watching: true });
  assert.deepEqual(
    requests.map(
      (request) => `${request.method} ${request.pathname} ${JSON.stringify(request.body)}`,
    ),
    [
      `PUT /api/questions/by-id/${encodeURIComponent(questionId)}/stewardship/star {"starred":true}`,
      `PUT /api/question-pools/${encodeURIComponent(poolId)}/stewardship/star {"starred":true}`,
      `PUT /api/questions/by-id/${encodeURIComponent(questionId)}/stewardship/watch {"watching":true}`,
      `PUT /api/question-pools/${encodeURIComponent(poolId)}/stewardship/watch {"watching":true}`,
    ],
  );

  const renderStarSurfaces = await loadStarSurfaceRenderer();
  const html = await renderStarSurfaces(
    {
      client: createHttpApiClient({
        fetch: async () =>
          new Response(
            JSON.stringify({
              starCount: 0,
              viewerHasStarred: false,
              starredInstructors: [],
            }),
            { headers: { "cache-control": "no-store", "content-type": "application/json" } },
          ),
      }),
      queries: {},
    },
    questionId,
    poolId,
  );
  assert.match(html, />Star question</);
  assert.match(html, />Star Pool</);
});

test("PLE always stores, transmits, displays, and copies the canonical hyphenated form.", async () => {
  const CopyableQuestionId = await loadCopyableQuestionId();
  const shown = renderToString(() =>
    createComponent(CopyableQuestionId, {
      questionTitle: "Peptide bond",
      displayId: "ABCD-XEFG",
    }),
  );
  assert.match(shown, /<code[^>]*>ABCD-XEFG<\/code>/);
  assert.match(shown, /Copy Question ID ABCD-XEFG/);
  assert.doesNotMatch(shown, /ABCDXEFG/);
  const hidden = renderToString(() =>
    createComponent(CopyableQuestionId, {
      questionTitle: "Peptide bond",
      displayId: "ABCDXEFG",
    }),
  );
  assert.match(hidden, /Question ID is unavailable/);
  assert.doesNotMatch(hidden, /Copy ID/);
});

async function loadCopyableQuestionId() {
  const result = await build({
    bundle: true,
    entryPoints: [new URL("../src/components/copyable_question_id.tsx", import.meta.url).pathname],
    format: "esm",
    outfile: "copyable_question_id.js",
    platform: "node",
    plugins: [solidPlugin({ solid: { generate: "ssr", hydratable: false } })],
    write: false,
    loader: { ".css": "empty" },
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined)
    throw new Error("Copyable Question ID bundle is missing JavaScript.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module.CopyableQuestionId !== "function") {
    throw new Error("Copyable Question ID bundle does not export the component.");
  }
  return module.CopyableQuestionId;
}

test("UUIDs should never appear in visible content, navigation URLs, or copyable links.", async () => {
  const uuid = "aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee";
  const surfaces = await loadVisibleIdentitySurfaces();
  const description = surfaces.blueprintClassificationDescription(
    {
      disciplineUuid: uuid,
      subjectUuid: null,
      topicUuid: null,
      subtopicUuid: null,
      crossDiscipline: false,
    },
    new Map(),
  );
  assert.equal(description, "Name unavailable");
  assert.doesNotMatch(description, /aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee/);
  const classification = await surfaces.renderClassification(uuid);
  assert.match(classification, /Discipline: Name unavailable/);
  assert.doesNotMatch(classification, /aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee/);
  const attempt = surfaces.archivedAttemptVisibleFacts(
    {
      courseInstanceId: "CI7K3M2QAZ",
      courseRosterTuple: { courseInstanceId: "CI7K3M2QAZ", rosterId: "RU-001" },
      assessmentId: "A9D2RX5AF",
      assessmentTitle: "Quiz",
      assessmentAttemptId: uuid,
      assessmentAttemptNumber: 2,
      startedAt: "2026-09-30T12:00:00.000Z",
      submittedAt: null,
      studentDataArchivedAt: "2026-10-01T12:00:00.000Z",
      deleteDueAt: "2027-01-01T12:00:00.000Z",
    },
    surfaces.formatDateTime,
  );
  const attemptText = `${attempt.title}\n${JSON.stringify(attempt.details)}`;
  assert.match(attemptText, /Quiz: Attempt 2/);
  assert.match(attemptText, /RU-001/);
  assert.doesNotMatch(attemptText, /aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee/);
  const notification = surfaces.notificationContent((timestamp) => String(timestamp))({
    targetKind: "question",
    targetPublicId: "7K3M-79QP",
    occurredAt: 1700000000000,
    eventKind: "impactNotice",
    revisionNumber: null,
    forkedPublicId: null,
    activityId: uuid,
  });
  const notificationText = `${notification.title}\n${notification.description}\n${JSON.stringify(notification.details)}`;
  assert.match(notificationText, /Impact notice activity/);
  assert.match(notificationText, /7K3M-79QP/);
  assert.doesNotMatch(notificationText, /aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee/);
  assert.equal(notification.actions.length, 1);
  assert.equal(notification.actions[0].href, "/library/7K3M-79QP");
  assert.equal(notification.actions[0].label, "Open Question");
  const poolNotification = surfaces.notificationContent((timestamp) => String(timestamp))({
    targetKind: "questionPool",
    targetPublicId: "3S8B-24DZ",
    occurredAt: 1700000000000,
    eventKind: "membersChanged",
    revisionNumber: 3,
    forkedPublicId: null,
    activityId: null,
  });
  assert.equal(poolNotification.actions[0].href, "/library/3S8B-24DZ");
  assert.equal(poolNotification.actions[0].label, "Open Question Pool");
});

async function loadVisibleIdentitySurfaces() {
  const result = await build({
    bundle: true,
    stdin: {
      contents: `
        import { createComponent, Suspense } from "solid-js";
        import { renderToStringAsync } from "solid-js/web";
        import { ApplicationApiProvider } from "./src/api/application_api.tsx";
        import { CourseClassificationSummary } from "./src/components/course_classification_summary.tsx";
        import { archivedAttemptVisibleFacts } from "./src/components/course_student_work_recovery.tsx";
        import { createDisplayDateTimeFormatter } from "./src/format_datetime.ts";
        import { blueprintClassificationDescription } from "./src/pages/blueprint_course_search_classification.tsx";
        import { notificationContent } from "./src/pages/library_watch_notifications_page.tsx";
        const formatDateTime = createDisplayDateTimeFormatter("UTC");
        function renderClassification(uuid) {
          return renderToStringAsync(() =>
            createComponent(Suspense, {
              get children() {
                return createComponent(ApplicationApiProvider, {
                  applicationApi: {
                    client: {
                      listDisciplinesIncludingRetired: () => Promise.resolve([]),
                      listSubjects: () => Promise.resolve([]),
                      listTopics: () => Promise.resolve([]),
                      listSubtopics: () => Promise.resolve([]),
                    },
                    queries: {},
                  },
                  get children() {
                    return createComponent(CourseClassificationSummary, {
                      value: {
                        disciplineUuid: uuid,
                        subjectUuid: null,
                        topicUuid: null,
                        subtopicUuid: null,
                        tags: [],
                      },
                    });
                  },
                });
              },
            }),
          );
        }
        export {
          archivedAttemptVisibleFacts,
          blueprintClassificationDescription,
          formatDateTime,
          notificationContent,
          renderClassification,
        };
      `,
      resolveDir: new URL("..", import.meta.url).pathname,
      sourcefile: "visible_identity_ssr.js",
      loader: "js",
    },
    format: "esm",
    outfile: "visible_identity_ssr.js",
    platform: "node",
    plugins: [
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
      solidPlugin({ solid: { generate: "ssr", hydratable: false } }),
    ],
    write: false,
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined)
    throw new Error("Visible identity SSR bundle is missing JavaScript.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module.renderClassification !== "function") {
    throw new Error("Visible identity SSR bundle has no renderer.");
  }
  return module;
}

test("the generated browser surface excludes answer-bearing type names", () => {
  const apiDirectory = path.resolve("generated/api");
  const forbidden = /\b(?:AnswerKey|CorrectResponse|SolutionKey)\b/;
  for (const filename of fs.readdirSync(apiDirectory).filter((name) => name.endsWith(".ts"))) {
    assert.doesNotMatch(
      fs.readFileSync(path.join(apiDirectory, filename), "utf8"),
      forbidden,
      filename,
    );
  }
});
