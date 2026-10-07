/** Shared PLE Question JSON authoring fixtures for codec tests. */

export function source() {
  return {
    format: "pleQuestionJson",
    prompt: "What is my favorite color?",
    response: {
      kind: "singleChoice",
      choices: [
        { id: "blue", text: "Blue", feedback: "Correct choice." },
        { id: "red", text: "Red", feedback: "Not this one." },
      ],
      correctChoice: "blue",
      randomizeChoices: false,
    },
    questionHint: "Compare each choice before responding.",
    feedback: { correct: "Exactly right.", incorrect: "Try again." },
    externalResources: [],
    authorScript: null,
  };
}

export function recordMetadata() {
  return {
    questionTitle: "Favorite color",
    questionDescription: "Instructor-facing color-choice example.",
    tags: ["example"],
    questionLicense: "CC-BY-SA-4.0",
    questionCitation: null,
    language: "en-US",
  };
}
