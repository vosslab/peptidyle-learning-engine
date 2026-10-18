// contract_pages.tsx - route surfaces whose application controls are still pending.

import { A, useParams } from "@solidjs/router";
import type { Component, JSX } from "solid-js";

import { PageFrame } from "../components/page_frame/page_frame";

function contractPage(surface: string, nextMilestone: string): Component {
  return function ContractPage(): JSX.Element {
    return (
      <PageFrame routeSurface={surface}>
        <p>Working controls for this route arrive in {nextMilestone}.</p>
        <A class="quiet-link" href="/">
          Return to courses
        </A>
      </PageFrame>
    );
  };
}

export const LibraryPage = contractPage("library", "the Question Library browser lane");
export const QuestionDetailPage = contractPage(
  "questionDetail",
  "the Question Library browser lane",
);
export function NotFoundPage(): JSX.Element {
  const params = useParams();
  return (
    <PageFrame routeSurface="notFound">
      <p>The requested path {params["unmatched"] ?? ""} is unavailable.</p>
      <A class="primary-link" href="/">
        Return to courses
      </A>
    </PageFrame>
  );
}
