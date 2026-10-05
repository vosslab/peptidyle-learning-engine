import type { Accessor, JSX } from "solid-js";

import { PageFrame, type PageFrameProps } from "../../components/page_frame";
import { SearchLeaveGuard } from "../../components/search_leave_guard";
import type { SearchStateController } from "./search_state";
import "./search.css";

export type SearchPageProps<Query, Row, FilterCounts, Sort extends string = never> = Omit<
  PageFrameProps,
  "children"
> & {
  readonly state: SearchStateController<Query, Row, FilterCounts, Sort>;
  /** Content-owned text that has not yet been submitted through the shared control. */
  readonly hasDraft?: Accessor<boolean>;
  /** Domain controls and filters placed in the shared toolbar column. */
  readonly toolbar?: JSX.Element;
  /** Domain rows, notices, and result actions placed in the shared results column. */
  readonly results?: JSX.Element;
  /** Dialogs remain mounted beside the main search surface while it is hidden for a task. */
  readonly extra?: JSX.Element;
};

/** Page shell for search surfaces: shared framing, responsive layout, and leave protection. */
export function SearchPage<Query, Row, FilterCounts, Sort extends string = never>(
  props: SearchPageProps<Query, Row, FilterCounts, Sort>,
): JSX.Element {
  const hasUnsavedSearch = (): boolean =>
    props.state.searchUsed() || props.state.hasTypedDraft() || props.hasDraft?.() === true;

  return (
    <PageFrame
      eyebrow={props.eyebrow}
      title={props.title}
      lede={props.lede}
      actions={props.actions}
      routeSurface={props.routeSurface}
      deniedRoute={props.deniedRoute}
      headingId={props.headingId}
      headingTabIndex={props.headingTabIndex}
      contentClass={props.contentClass}
    >
      <div
        class="shared-search-page"
        classList={{ "shared-search-page--initial": props.state.state().kind === "initial" }}
      >
        <div class="shared-search-page__toolbar">{props.toolbar}</div>
        <div class="shared-search-page__results">{props.results}</div>
      </div>
      {props.extra}
      <SearchLeaveGuard hasSearch={hasUnsavedSearch} />
    </PageFrame>
  );
}
