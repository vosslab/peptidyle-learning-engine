// ribbon_m9_responsive_harness.tsx - real DOM seam for responsive Ribbon evidence.

import { createSignal } from "solid-js";
import { render } from "solid-js/web";

import type { RibbonDestinationId } from "../../src/ribbon/ribbon_catalog";
import type { RibbonTabId } from "../../src/route_contract";
import { AppRibbon } from "../../src/ribbon/app_ribbon";
import type { RibbonModel } from "../../src/ribbon/ribbon_contract";
import { M6_RIBBON_FIXTURES, type M6RibbonFixtureName } from "./ribbon_model_fixtures";

function selectTab(model: RibbonModel, tabId: RibbonTabId): RibbonModel {
  if (!model.tabs.some((tab) => tab.id === tabId)) {
    throw new Error(`Responsive harness cannot select absent Tab ${tabId}.`);
  }
  return {
    ...model,
    tabs: model.tabs.map((tab) => ({ ...tab, selected: tab.id === tabId })),
  };
}

function selectTask(model: RibbonModel, taskId: RibbonDestinationId): RibbonModel {
  const hasTask = model.taskAreas.some((area) => area.controls.some((task) => task.id === taskId));
  if (!hasTask) throw new Error(`Responsive harness cannot select absent Task ${taskId}.`);
  return {
    ...model,
    taskAreas: model.taskAreas.map((area) => ({
      ...area,
      controls: area.controls.map((task) => ({ ...task, selected: task.id === taskId })),
    })),
  };
}

export interface RibbonM9ResponsiveHarness {
  readonly dispose: () => void;
  readonly selectTask: (taskId: RibbonDestinationId) => void;
  readonly selectTab: (tabId: RibbonTabId) => void;
  readonly setFixture: (fixture: M6RibbonFixtureName) => void;
}

/** Mounts the compiled Ribbon without an application, session, or route runtime. */
export function mountRibbonM9ResponsiveHarness(target: HTMLElement): RibbonM9ResponsiveHarness {
  const [model, setModel] = createSignal<RibbonModel>(M6_RIBBON_FIXTURES.courseInstructor);
  // The corpus contexts emulate reduced motion; make that shell-owned user
  // preference explicit rather than letting a static fixture silently test
  // the ordinary smooth-scroll default.
  const dispose = render(() => <AppRibbon model={model()} reducedMotion={() => true} />, target);
  return {
    dispose,
    selectTask: (taskId) => setModel((current) => selectTask(current, taskId)),
    selectTab: (tabId) => setModel((current) => selectTab(current, tabId)),
    setFixture: (fixture) => setModel(M6_RIBBON_FIXTURES[fixture]),
  };
}
