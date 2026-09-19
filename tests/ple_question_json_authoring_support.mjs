/** Shared PLE Question JSON authoring fixtures for codec tests. */

export function source() {
  return {
    format: "pleQuestionJson",
    questionTitle: "Favorite color",
    questionDescription: "Instructor-facing color-choice example.",
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
    tags: ["example"],
    questionLicense: "CC-BY-SA-4.0",
    questionCitation: null,
    externalResources: [],
    authorScript: null,
    language: "en-US",
  };
}
