// page_frame.tsx - shared page layout and content composition.

import { Show, type JSX } from "solid-js";

import { useRouteContentLayout } from "../ribbon/route_scope_context";
import "./page_frame.css";

export interface PageFrameProps {
  readonly actions?: JSX.Element;
  readonly children?: JSX.Element;
  readonly routeSurface?: string;
  readonly deniedRoute?: string;
  /**
   * Task-specific inner arrangement only. PageFrame owns outer geometry and the
   * ordinary content stack. Keep this when the class scopes an editor, table,
   * or failure card; omit it when it only repeated that stack.
   */
  readonly contentClass?: string;
}

function pageFrameContentClassName(contentClass: string | undefined): string {
  const baseClassName = "page-frame__content";
  const resolvedClassName =
    contentClass === undefined ? baseClassName : `${baseClassName} ${contentClass}`;
  return resolvedClassName;
}

/**
 * Gives every route the declared content width and ordinary content stack.
 * The Ribbon and breadcrumbs identify the page; callers own task content.
 */
export function PageFrame(props: PageFrameProps): JSX.Element {
  const contentLayout = useRouteContentLayout();

  return (
    <section
      class="page-frame"
      classList={{ "page-frame--full-width": contentLayout() === "fullWidth" }}
      data-content-layout={contentLayout()}
      data-denied-route={props.deniedRoute}
      data-route-surface={props.routeSurface}
    >
      <Show when={props.actions}>
        {(actions) => <div class="page-frame__actions">{actions()}</div>}
      </Show>
      <div class={pageFrameContentClassName(props.contentClass)}>{props.children}</div>
    </section>
  );
}
