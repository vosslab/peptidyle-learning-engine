// contract_pages.tsx - route surfaces whose application controls are still pending.

import { A, useParams } from "@solidjs/router";
import type { Component, JSX } from "solid-js";

import { PageFrame } from "../components/page_frame";

function contractPage(surface: string, title: string, nextMilestone: string): Component {
  return function ContractPage(): JSX.Element {
    return (
      <PageFrame
        routeSurface={surface}
        eyebrow="Architecture contract"
        title={title}
        lede={
          <>
            This route is wired and keeps the application shell active. Its working controls arrive
            in {nextMilestone}.
          </>
        }
      >
        <A class="quiet-link" href="/">
          Return to courses
        </A>
      </PageFrame>
    );
  };
}

export const LibraryPage = contractPage(
  "library",
  "Question Library",
  "the Question Library browser lane",
);
export const QuestionDetailPage = contractPage(
  "questionDetail",
  "Question Revision",
  "the Question Library browser lane",
);
export function NotFoundPage(): JSX.Element {
  const params = useParams();
  return (
    <PageFrame
      routeSurface="notFound"
      eyebrow="Route not found"
      title="That page is not part of this learning space"
      lede={<>The requested path {params["unmatched"] ?? ""} is unavailable.</>}
    >
      <A class="primary-link" href="/">
        Return to courses
      </A>
    </PageFrame>
  );
}
