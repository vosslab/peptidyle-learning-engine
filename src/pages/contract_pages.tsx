// contract_pages.tsx - honest route surfaces reserved by WP-C9 for later lanes.

import { A, useParams } from "@solidjs/router";
import type { Component, JSX } from "solid-js";

function contractPage(surface: string, title: string, nextMilestone: string): Component {
  return function ContractPage(): JSX.Element {
    return (
      <section class="page" data-route-surface={surface}>
        <p class="eyebrow">Architecture contract</p>
        <h1>{title}</h1>
        <p class="page-lede">
          This route is wired and keeps the application shell active. Its working controls arrive in{" "}
          {nextMilestone}.
        </p>
        <A class="quiet-link" href="/">
          Return to courses
        </A>
      </section>
    );
  };
}

export const RunSummaryPage = contractPage("runSummary", "Run summary", "the attempt-loop lane");
export const LibraryPage = contractPage("library", "Problem library", "the catalog-browser lane");
export const ProblemDetailPage = contractPage(
  "problemDetail",
  "Problem version",
  "the catalog-browser lane",
);
export const WorkspaceListPage = contractPage(
  "workspaceList",
  "Instructor workspace",
  "the editor lane",
);
export const WorkspaceEditorPage = contractPage(
  "workspaceEditor",
  "Draft editor",
  "the editor lane",
);
export function NotFoundPage(): JSX.Element {
  const params = useParams();
  return (
    <section class="page" data-route-surface="notFound">
      <p class="eyebrow">Route not found</p>
      <h1>That page is not part of this learning space</h1>
      <p class="page-lede">The requested path {params["unmatched"] ?? ""} is unavailable.</p>
      <A class="primary-link" href="/">
        Return to courses
      </A>
    </section>
  );
}
