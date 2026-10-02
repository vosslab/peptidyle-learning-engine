// Question Library search is the collection workflow. Archive stays one Published Question.

import assert from "node:assert/strict";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

import { publishedQuestionFixture } from "./fixtures/published_question.ts";

const question = publishedQuestionFixture.publishedQuestion;

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
                                searchQuestionLibrary: unavailable("searchQuestionLibrary"),
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
  assert.match(libraryHtml, /Search published questions/);
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
});
