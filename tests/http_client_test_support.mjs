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
export function issuedQuestionWireFixture(attempt, publishedQuestionRevision) {
  const response = publishedQuestionRevision.response;
  if (response.kind !== "multipleChoice")
    throw new Error("fixture requires multiple-choice response");
  return {
    questionRevision: { questionId: "7K3-M9QP", revisionNumber: 1 },
    question_seed: attempt.question_seed,
    presentationNonce: attempt.id.replaceAll("-", "").slice(-32),
    title: publishedQuestionRevision.metadata.title,
    prompt: publishedQuestionRevision.prompt.map((block) =>
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
export async function validateSavedResponse(responseFormat, response) {
  if (responseFormat.kind === "multipleChoice" && response.kind === "multipleChoice") {
    const validIds = new Set(responseFormat.choices.map((choice) => choice.id));
    const unique = new Set(response.selected);
    return unique.size === response.selected.length &&
      response.selected.every((id) => validIds.has(id))
      ? { issues: [] }
      : { issues: [{ kind: "unknownChoice", choice: "invalid-selection" }] };
  }
  if (responseFormat.kind === "ordering" && response.kind === "ordering") {
    const expected = new Set(responseFormat.items.map((item) => item.id));
    const actual = new Set(response.order);
    return actual.size === responseFormat.items.length &&
      [...actual].every((id) => expected.has(id))
      ? { issues: [] }
      : { issues: [{ kind: "orderingItemsMismatch" }] };
  }
  return { issues: [{ kind: "responseKindMismatch" }] };
}
