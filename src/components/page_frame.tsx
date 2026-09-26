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
  /**
   * Task-specific inner arrangement only. PageFrame owns outer geometry and the
   * ordinary content stack. Keep this when the class scopes an editor, table,
   * or failure card; omit it when it only repeated that stack.
   */
  readonly contentClass?: string;
}

export interface PageSectionProps {
  readonly heading: JSX.Element;
  readonly headingId?: string;
  readonly helper?: JSX.Element;
  readonly actions?: JSX.Element;
  readonly children?: JSX.Element;
}

function pageFrameContentClassName(contentClass: string | undefined): string {
  const baseClassName = "page-frame__content";
  const resolvedClassName =
    contentClass === undefined ? baseClassName : `${baseClassName} ${contentClass}`;
  return resolvedClassName;
}

/**
 * Gives every route the declared content width, one page heading, and the ordinary
 * content stack. Route-specific page content remains in the caller's children.
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

/** In-page section: heading, optional helper and actions, then the section body. */
export function PageSection(props: PageSectionProps): JSX.Element {
  return (
    <section class="page-section" aria-labelledby={props.headingId}>
      <header class="page-section__header">
        <h2 class="page-section__heading" id={props.headingId}>
          {props.heading}
        </h2>
        <Show when={props.helper}>
          <p class="page-section__helper">{props.helper}</p>
        </Show>
      </header>
      <Show when={props.actions}>
        {(actions) => <div class="page-section__actions">{actions()}</div>}
      </Show>
      <Show when={props.children}>
        <div class="page-section__body">{props.children}</div>
      </Show>
    </section>
  );
}
