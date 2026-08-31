// Test-local, literal HTTP fixtures for focused client-boundary tests.

export function jsonResponse(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json; charset=utf-8" },
  });
}

export function createRecordingFetch(respond) {
  const requests = [];

  async function recordingFetch(input, init) {
    const request = new Request(new URL(input.toString(), "https://client.example.test"), init);
    requests.push(request.clone());
    const response = await respond(request);
    return response;
  }

  return { recordingFetch, requests };
}

/** Builds a literal browser-issued presentation from an already-public test attempt. */
export function issuedQuestionWireFixture(attempt, publishedProblem) {
  const response = publishedProblem.response;
  if (response.kind !== "multipleChoice")
    throw new Error("fixture requires multiple-choice response");
  return {
    questionVersion: { questionId: "7K3-M9QP", versionNumber: 1 },
    seed: attempt.seed,
    presentationNonce: attempt.id.replaceAll("-", "").slice(-32),
    title: publishedProblem.metadata.title,
    prompt: publishedProblem.prompt.map((block) =>
      block.kind === "text"
        ? { ...block, markdown: block.markdown.replace("{{residue}}", "glycine") }
        : block,
    ),
    response: {
      kind: "singleChoice",
      choices: response.choices.map((choice, index) => ({
        id: (index + 1).toString(16).padStart(4, "0"),
        body: choice.body,
      })),
    },
  };
}

/** Tests attempt recovery without importing the retired application validator. */
export async function validateSavedResponse(definition, response) {
  if (definition.kind === "multipleChoice" && response.kind === "multipleChoice") {
    const validIds = new Set(definition.choices.map((choice) => choice.id));
    const unique = new Set(response.selected);
    return unique.size === response.selected.length &&
      response.selected.every((id) => validIds.has(id))
      ? { violations: [] }
      : { violations: [{ kind: "invalidSelection" }] };
  }
  if (definition.kind === "ordering" && response.kind === "ordering") {
    const expected = new Set(definition.items.map((item) => item.id));
    const actual = new Set(response.order);
    return actual.size === definition.items.length && [...actual].every((id) => expected.has(id))
      ? { violations: [] }
      : { violations: [{ kind: "invalidOrder" }] };
  }
  return { violations: [{ kind: "responseKindMismatch" }] };
}
