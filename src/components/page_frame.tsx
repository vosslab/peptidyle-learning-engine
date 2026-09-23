// page_frame.tsx - shared page width and heading composition.

import { Show, type JSX } from "solid-js";

import { useRouteContentLayout } from "../ribbon/route_scope_context";
import "./page_frame.css";

export interface PageFrameProps {
  readonly eyebrow?: JSX.Element;
  readonly title: JSX.Element;
  readonly lede?: JSX.Element;
  readonly actions?: JSX.Element;
  readonly children?: JSX.Element;
  readonly routeSurface?: string;
  readonly deniedRoute?: string;
  readonly headingId?: string;
  readonly headingTabIndex?: number;
  readonly contentClass?: string;
}

function pageFrameContentClassName(contentClass: string | undefined): string {
  const baseClassName = "page-frame__content";
  const resolvedClassName =
    contentClass === undefined ? baseClassName : `${baseClassName} ${contentClass}`;
  return resolvedClassName;
}

/**
 * Gives every route the declared content width and one consistent page heading shape.
 * Route-specific page content remains in the caller's children.
 */
export function PageFrame(props: PageFrameProps): JSX.Element {
  const contentLayout = useRouteContentLayout();

  return (
    <section
      class="page-frame"
      classList={{ "page-frame--full-width": contentLayout() === "fullWidth" }}
      aria-labelledby={props.headingId}
      data-content-layout={contentLayout()}
      data-denied-route={props.deniedRoute}
      data-route-surface={props.routeSurface}
    >
      <header class="page-frame__header">
        <Show when={props.eyebrow}>
          <p class="page-frame__eyebrow eyebrow">{props.eyebrow}</p>
        </Show>
        <h1 class="page-frame__title" id={props.headingId} tabindex={props.headingTabIndex}>
          {props.title}
        </h1>
        <Show when={props.lede}>
          <p class="page-frame__lede page-lede">{props.lede}</p>
        </Show>
      </header>
      <Show when={props.actions}>
        {(actions) => <div class="page-frame__actions">{actions()}</div>}
      </Show>
      <div class={pageFrameContentClassName(props.contentClass)}>{props.children}</div>
    </section>
  );
}
