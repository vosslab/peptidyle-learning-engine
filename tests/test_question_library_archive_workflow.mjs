// Question Library search is the collection workflow. Archive stays one Published Question.

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

const question = {
  questionId: "7K3M-79QP",
  metadata: { questionTitle: "Peptide bond resonance" },
};
const questionDetailSource = readFileSync(
  new URL("../src/pages/question_detail_page.tsx", import.meta.url),
  "utf8",
);

async function loadArchiveWorkflowRenderer() {
  const result = await build({
    bundle: true,
    stdin: {
      contents: `
        import { createComponent, Suspense } from "solid-js";
        import { renderToStringAsync } from "solid-js/web";
        import { RouterContext } from "@solidjs/router";
        import { ApplicationApiProvider } from "./src/api/application_api.tsx";
        import { SessionContext } from "./src/auth/session_context.tsx";
        import { LibraryPage } from "./src/pages/library_page.tsx";
        import { QuestionArchiveControl } from "./src/pages/question_detail_page.tsx";
        import { RouteScopeProvider } from "./src/ribbon/route_scope_context.tsx";

        function unavailable(name) {
          return async () => {
            throw new Error(name + " is outside this Question Library search render");
          };
        }

        const libraryLocation = {
          pathname: "/questions/search",
          search: "",
          hash: "",
          query: {},
          state: null,
          key: "",
        };

        export async function renderLibrarySearch(session) {
          const bootstrap = {
            state: () => ({ kind: "authenticated", session }),
            retry: async () => undefined,
            signOut: async () => true,
          };
          const router = {
            location: libraryLocation,
            navigatorFactory: () => () => undefined,
            beforeLeave: { subscribe: () => () => undefined },
            get pendingTarget() {
              return undefined;
            },
            base: {
              path: () => "/",
              resolvePath: (to) => to,
            },
          };
          const applicationApi = {
            client: {},
            queries: {
              courseScope: unavailable("courseScope"),
              assessmentAttemptScope: unavailable("assessmentAttemptScope"),
              assessmentAttemptHistory: unavailable("assessmentAttemptHistory"),
            },
          };
          return renderToStringAsync(() =>
            createComponent(ApplicationApiProvider, {
              applicationApi,
              get children() {
                return createComponent(RouteScopeProvider, {
                  pathname: libraryLocation.pathname,
                  get children() {
                    return createComponent(SessionContext.Provider, {
                      value: bootstrap,
                      get children() {
                        return createComponent(RouterContext.Provider, {
                          value: router,
                          get children() {
                            return createComponent(LibraryPage, {
                              mode: "search",
                              repository: {
                                searchLibraryObjects: unavailable("searchLibraryObjects"),
                              },
                              metadataClient: {
                                replaceQuestionMetadata: unavailable("replaceQuestionMetadata"),
                              },
                              classificationClient: {
                                listDisciplines: unavailable("listDisciplines"),
                                listSubjects: unavailable("listSubjects"),
                                listTopics: unavailable("listTopics"),
                                listSubtopics: unavailable("listSubtopics"),
                              },
                              questionPoolClient: {
                                createQuestionPool: unavailable("createQuestionPool"),
                              },
                              getQuestionDetails: unavailable("getQuestionDetails"),
                            });
                          },
                        });
                      },
                    });
                  },
                });
              },
            }),
          );
        }

        export function renderQuestionArchive(client, questionId) {
          return renderToStringAsync(() =>
            createComponent(Suspense, {
              get children() {
                return createComponent(QuestionArchiveControl, { client, questionId });
              },
            }),
          );
        }
      `,
      resolveDir: new URL("..", import.meta.url).pathname,
      sourcefile: "question_library_archive_workflow_ssr.js",
      loader: "js",
    },
    format: "esm",
    outfile: "question_library_archive_workflow_ssr.js",
    platform: "node",
    conditions: ["solid"],
    write: false,
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
  });
  const javascript = result.outputFiles.find((output) => output.path.endsWith(".js"));
  if (javascript === undefined) {
    throw new Error("Question Library archive workflow SSR bundle is missing JavaScript.");
  }
  const encoded = Buffer.from(javascript.contents).toString("base64");
  return import(`data:text/javascript;base64,${encoded}`);
}

test("With 13,000 Questions in Neil's first course, manually archiving Questions is unlikely to be a useful primary workflow.", async () => {
  const renderer = await loadArchiveWorkflowRenderer();
  const libraryHtml = await renderer.renderLibrarySearch({
    authenticated: true,
    account: { id: "U0000035E", userRole: "instructor" },
  });
  assert.match(libraryHtml, /Search Question Library/);
  assert.match(libraryHtml, /Search Question Library/);
  assert.match(libraryHtml, /Create Question Pool/);
  assert.doesNotMatch(libraryHtml, /Archive/);

  const calls = [];
  const client = {
    async getQuestionLineage(questionId) {
      calls.push(["lineage", questionId]);
      return {
        summary: {
          questionId,
          availability: { availability: "available" },
          metadata: { questionTitle: question.metadata.questionTitle },
        },
        viewerMayArchive: true,
        viewerMayEditMetadata: true,
        questionAvailabilityEditNumber: "5",
      };
    },
    async archiveQuestion(questionId, confirmationTitle, editNumber) {
      calls.push(["archive", questionId, confirmationTitle, editNumber]);
      throw new Error("Rendering the archive control must not archive the Question");
    },
  };
  const archiveHtml = await renderer.renderQuestionArchive(client, question.questionId);
  assert.match(archiveHtml, /Archive Published Question/);
  assert.equal(archiveHtml.match(/Archive Published Question/g)?.length, 1);
  assert.doesNotMatch(archiveHtml, /Danger Zone/);
  assert.deepEqual(calls, [["lineage", question.questionId]]);

  const archivedOwnerClient = {
    async getQuestionLineage(questionId) {
      return {
        summary: {
          questionId,
          availability: { availability: "archived" },
          metadata: { questionTitle: question.metadata.questionTitle },
        },
        viewerMayArchive: true,
        viewerMayEditMetadata: false,
        questionAvailabilityEditNumber: "6",
      };
    },
    async archiveQuestion() {
      throw new Error("An archived Question must not be archived again");
    },
    async restoreQuestion() {
      throw new Error("Rendering the restore control must not restore the Question");
    },
  };
  const archivedOwnerHtml = await renderer.renderQuestionArchive(
    archivedOwnerClient,
    question.questionId,
  );
  assert.match(archivedOwnerHtml, /archived and read-only/);
  assert.match(archivedOwnerHtml, /Restore Published Question/);

  const archivedNonOwnerClient = {
    async getQuestionLineage(questionId) {
      return {
        summary: {
          questionId,
          availability: { availability: "archived" },
          metadata: { questionTitle: question.metadata.questionTitle },
        },
        viewerMayArchive: false,
        viewerMayEditMetadata: false,
        questionAvailabilityEditNumber: "6",
      };
    },
  };
  const archivedHtml = await renderer.renderQuestionArchive(
    archivedNonOwnerClient,
    question.questionId,
  );
  assert.match(archivedHtml, /archived and read-only/);
  assert.match(archivedHtml, /normal Question Library discovery/);
  assert.doesNotMatch(archivedHtml, /unavailable for new selection/);
  assert.doesNotMatch(archivedHtml, /Restore Published Question/);
  assert.doesNotMatch(archivedHtml, /Archive Published Question/);
});

test("the detail action boundary offers archived exact-Revision forks to every Instructor", () => {
  assert.match(
    questionDetailSource,
    /const mayMutateLibrary = \(\): boolean => \{\s*const state = session\.state\(\);\s*return state\.kind === "authenticated" && state\.session\.account\.userRole === "instructor";/u,
  );

  const archiveControlStart = questionDetailSource.indexOf(
    "<QuestionArchiveControl",
    questionDetailSource.indexOf("export function QuestionDetailPage"),
  );
  const archiveControlEnd = questionDetailSource.indexOf(
    "/>\n              <Show when={mayMutateLibrary()}>",
    archiveControlStart,
  );
  assert.notEqual(archiveControlStart, -1);
  assert.notEqual(archiveControlEnd, -1);
  const archiveControl = questionDetailSource.slice(archiveControlStart, archiveControlEnd);
  assert.match(
    archiveControl,
    /renderAvailableAction=\{\(\) => \([\s\S]*?<Show when=\{mayMutateLibrary\(\)\}>\s*<QuestionPoolFromQuestionControl\s+detail=\{record\(\)\}\s*\/>\s*<\/Show>[\s\S]*?\)\}/u,
  );
  const availableActionStart = archiveControl.indexOf("renderAvailableAction={() => (");
  const availableActionEnd = archiveControl.indexOf("\n                )}", availableActionStart);
  assert.notEqual(availableActionStart, -1);
  assert.notEqual(availableActionEnd, -1);
  const availableAction = archiveControl.slice(availableActionStart, availableActionEnd);
  assert.match(availableAction, /QuestionPoolFromQuestionControl/u);
  assert.doesNotMatch(availableAction, /QuestionForkControl/u);

  const forkAction = questionDetailSource.slice(archiveControlEnd + 3);
  assert.match(
    forkAction,
    /<Show when=\{mayMutateLibrary\(\)\}>\s*<QuestionForkControl\s+sourceRevisionTuple=\{record\(\)\.summary\.publishedQuestionRevisionTuple\}\s*\/>\s*<\/Show>/u,
  );
  assert.doesNotMatch(archiveControl, /QuestionForkControl/u);
});
