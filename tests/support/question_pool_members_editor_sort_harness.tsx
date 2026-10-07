// Browser harness for editable Pool draft sorting and Save payload stability.

import { render } from "solid-js/web";

import "../../src/browser_environment";

import type { QuestionPoolView } from "../../generated/api/QuestionPoolView";
import type { SaveQuestionPoolMembersInput } from "../../src/api/question_pool_members";
import type { QuestionPoolMembersEditorProps } from "../../src/components/question_pool_members_editor";
import { QuestionPoolMembersEditor } from "../../src/components/question_pool_members_editor";

declare global {
  interface Window {
    __pleQuestionPoolSaveRequests?: ReadonlyArray<SaveQuestionPoolMembersInput>;
  }
}

const pool = {
  questionPoolId: "3S8B-24DZ",
  questionPoolEditNumber: 7,
  metadata: { disciplineUuid: "discipline", subjectUuid: "subject" },
  questionType: "multipleChoice",
  backend: "native",
  members: [
    member("7K3M-79QP", 2, "Zulu"),
    member("8B7D-3C9F", 1, "Alpha"),
    member("4P2N-6W1Q", 4, "Mike"),
  ],
} as unknown as QuestionPoolView;

function member(questionId: string, revisionNumber: number, questionTitle: string): unknown {
  return {
    publishedQuestionRevisionTuple: { publishedQuestionId: questionId, revisionNumber },
    question: {
      question_library: {
        summary: {
          metadata: { questionTitle, questionLicense: "CC0-1.0" },
        },
      },
    },
  };
}

export function mountQuestionPoolMembersEditorSortHarness(target: HTMLElement): void {
  const requests: SaveQuestionPoolMembersInput[] = [];
  const props = {
    pool,
    client: {
      saveQuestionPoolMembers: (input: SaveQuestionPoolMembersInput) => {
        requests.push(input);
        return Promise.resolve({ questionPoolId: pool.questionPoolId, questionPoolEditNumber: 8 });
      },
    },
    questionLibrary: {},
    getQuestionDetails: () =>
      Promise.reject(new Error("The browser contract does not open the Question Picker.")),
    getCurrentQuestionSharedMetadata: () =>
      Promise.reject(new Error("The browser contract does not open the Question Picker.")),
    reloadPool: () => Promise.resolve(pool),
    onSaved: () => undefined,
  } as unknown as QuestionPoolMembersEditorProps;

  window.__pleQuestionPoolSaveRequests = requests;
  render(() => <QuestionPoolMembersEditor {...props} />, target);
}
