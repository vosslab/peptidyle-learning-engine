import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";

import { build } from "esbuild";
import { solidPlugin } from "esbuild-plugin-solid";

import { DecodeError } from "../src/api/decoder.ts";
import { createHttpApiClient } from "../src/api/http_client.ts";

const question = { questionId: "7K3M-79QP" };

function noStoreJson(value) {
  return new Response(JSON.stringify(value), {
    headers: { "cache-control": "no-store", "content-type": "application/json" },
  });
}

test("Question Watch client uses one closed self-only endpoint", async () => {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(input.toString(), "https://ple.example"), init);
      requests.push(request.clone());
      return noStoreJson({ watching: request.method === "PUT" });
    },
  });

  assert.deepEqual(await client.getQuestionWatch(question.questionId), { watching: false });
  assert.deepEqual(await client.setQuestionWatch(question.questionId, true), { watching: true });
  assert.equal(
    new URL(requests[0].url).pathname,
    `/api/questions/by-id/${encodeURIComponent(question.questionId)}/stewardship/watch`,
  );
  assert.equal(requests[0].method, "GET");
  assert.equal(requests[1].method, "PUT");
  assert.deepEqual(await requests[1].json(), { watching: true });
});

test("Question Watch client rejects any watcher disclosure beyond its boolean", async () => {
  const client = createHttpApiClient({
    fetch: async () => noStoreJson({ watching: true, watcherCount: 1 }),
  });
  await assert.rejects(client.getQuestionWatch(question.questionId), DecodeError);
});

const POOL_ID = "3S8B-24DZ";

async function loadWatchSurfaceRenderer() {
  const result = await build({
    bundle: true,
    stdin: {
      contents: `
        import { createComponent, Suspense } from "solid-js";
        import { renderToStringAsync } from "solid-js/web";
        import { ApplicationApiProvider } from "./src/api/application_api.tsx";
        import { QuestionPoolWatchControl } from "./src/components/question_pool_watch_control.tsx";
        import { QuestionWatchControl } from "./src/components/question_watch_control.tsx";
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
        export function renderWatchSurfaces(applicationApi, questionId, poolId) {
          return Promise.all([
            renderControl(applicationApi, QuestionWatchControl, { questionId }),
            renderControl(applicationApi, QuestionPoolWatchControl, { poolId }),
          ]).then(([questionHtml, poolHtml]) => questionHtml + poolHtml);
        }
      `,
      resolveDir: new URL("..", import.meta.url).pathname,
      sourcefile: "watch_surface_ssr.js",
      loader: "js",
    },
    format: "esm",
    outfile: "watch_surface_ssr.js",
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
  if (javascript === undefined) throw new Error("Watch surface SSR bundle is missing JavaScript.");
  const encoded = Buffer.from(javascript.contents).toString("base64");
  const module = await import(`data:text/javascript;base64,${encoded}`);
  if (typeof module.renderWatchSurfaces !== "function") {
    throw new Error("Watch surface SSR bundle has no renderer.");
  }
  return module.renderWatchSurfaces;
}

function watchClient(watching) {
  const requests = [];
  const client = createHttpApiClient({
    fetch: async (input, init) => {
      const request = new Request(new URL(String(input), "https://ple.example"), init);
      const body = request.method === "GET" ? null : await request.json();
      requests.push({ method: request.method, pathname: new URL(request.url).pathname, body });
      return noStoreJson({ watching: request.method === "PUT" ? body.watching : watching });
    },
  });
  return { client, requests };
}

test("Watch means subscription.", async () => {
  const detail = fs.readFileSync(
    new URL("../src/pages/question_detail_page.tsx", import.meta.url),
    "utf8",
  );
  const poolDetail = fs.readFileSync(
    new URL("../src/pages/question_pool_detail.tsx", import.meta.url),
    "utf8",
  );
  assert.match(
    detail,
    /<Show when=\{mayMutateLibrary\(\)\}>[\s\S]*<QuestionWatchControl questionId=\{record\(\)\.summary\.questionId\} \/>/,
  );
  assert.match(
    poolDetail,
    /<Show when=\{mayMutateLibrary\(\)\}>[\s\S]*<QuestionPoolWatchControl poolId=\{value\(\)\.questionPoolId\} \/>/,
  );

  const renderWatchSurfaces = await loadWatchSurfaceRenderer();
  const unsubscribed = watchClient(false);
  const unsubscribedHtml = await renderWatchSurfaces(
    { client: unsubscribed.client, queries: {} },
    question.questionId,
    POOL_ID,
  );
  assert.match(unsubscribedHtml, />Watch question</);
  assert.match(unsubscribedHtml, />Watch Pool</);
  assert.doesNotMatch(unsubscribedHtml, /Unwatch/);
  assert.doesNotMatch(unsubscribedHtml, /watcher/i);

  const subscribed = watchClient(true);
  const subscribedHtml = await renderWatchSurfaces(
    { client: subscribed.client, queries: {} },
    question.questionId,
    POOL_ID,
  );
  assert.match(subscribedHtml, />Unwatch question</);
  assert.match(subscribedHtml, />Unwatch Pool</);
  assert.match(subscribedHtml, /aria-pressed="true"/);

  const subscription = watchClient(false);
  assert.deepEqual(await subscription.client.setQuestionWatch(question.questionId, true), {
    watching: true,
  });
  assert.deepEqual(await subscription.client.setQuestionPoolWatch(POOL_ID, true), {
    watching: true,
  });
  assert.deepEqual(await subscription.client.setQuestionWatch(question.questionId, false), {
    watching: false,
  });
  assert.deepEqual(await subscription.client.setQuestionPoolWatch(POOL_ID, false), {
    watching: false,
  });
  assert.deepEqual(
    subscription.requests.map(
      (request) => `${request.method} ${request.pathname} ${JSON.stringify(request.body)}`,
    ),
    [
      `PUT /api/questions/by-id/${encodeURIComponent(question.questionId)}/stewardship/watch {"watching":true}`,
      `PUT /api/question-pools/${encodeURIComponent(POOL_ID)}/stewardship/watch {"watching":true}`,
      `PUT /api/questions/by-id/${encodeURIComponent(question.questionId)}/stewardship/watch {"watching":false}`,
      `PUT /api/question-pools/${encodeURIComponent(POOL_ID)}/stewardship/watch {"watching":false}`,
    ],
  );

  const disclosing = createHttpApiClient({
    fetch: async () => noStoreJson({ watching: true, watcherCount: 4 }),
  });
  await assert.rejects(disclosing.getQuestionPoolWatch(POOL_ID), DecodeError);
});

test("An Instructor's watch list remains private.", async () => {
  const otherInstructorWatchList = [
    {
      publishedQuestionId: question.questionId,
      instructorAccountId: "U0000002Y",
      displayName: "Ada Lopez",
    },
  ];
  const questionLeak = createHttpApiClient({
    fetch: async () => noStoreJson({ watching: true, watchList: otherInstructorWatchList }),
  });
  await assert.rejects(questionLeak.getQuestionWatch(question.questionId), DecodeError);

  const poolLeak = createHttpApiClient({
    fetch: async () =>
      noStoreJson({
        watching: false,
        watchers: [{ displayName: "Grace Hopper", instructorAccountId: "U0000001X" }],
      }),
  });
  await assert.rejects(poolLeak.getQuestionPoolWatch(POOL_ID), DecodeError);

  const inboxRequests = [];
  const inboxLeak = createHttpApiClient({
    fetch: async (input, init) => {
      inboxRequests.push(new Request(new URL(String(input), "https://ple.example"), init));
      return noStoreJson({ notifications: [], watchList: otherInstructorWatchList });
    },
  });
  await assert.rejects(inboxLeak.getLibraryWatchNotifications(), DecodeError);
  assert.equal(new URL(inboxRequests[0].url).pathname, "/api/library/watch-notifications");

  const renderWatchSurfaces = await loadWatchSurfaceRenderer();
  const ownList = watchClient(true);
  const html = await renderWatchSurfaces(
    { client: ownList.client, queries: {} },
    question.questionId,
    POOL_ID,
  );
  assert.match(html, />Unwatch question</);
  assert.match(html, />Unwatch Pool</);
  assert.doesNotMatch(html, /Ada Lopez|Grace Hopper|watch list|watchers|U0000002Y/);

  const closed = watchClient(true);
  assert.deepEqual(await closed.client.getQuestionWatch(question.questionId), { watching: true });
  assert.deepEqual(await closed.client.getQuestionPoolWatch(POOL_ID), { watching: true });
  assert.deepEqual(
    closed.requests.map((request) => `${request.method} ${request.pathname}`),
    [
      `GET /api/questions/by-id/${encodeURIComponent(question.questionId)}/stewardship/watch`,
      `GET /api/question-pools/${encodeURIComponent(POOL_ID)}/stewardship/watch`,
    ],
  );
});
