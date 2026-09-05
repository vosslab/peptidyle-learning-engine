// ribbon_m8_integration_harness.tsx - real DOM mount seam for Ribbon interaction evidence.

import { createSignal } from "solid-js";
import { render } from "solid-js/web";

import type { RibbonTabId } from "../../src/route_contract";
import { AppRibbon } from "../../src/ribbon/app_ribbon";
import type { RibbonModel } from "../../src/ribbon/ribbon_contract";
import { M6_RIBBON_FIXTURES } from "./ribbon_model_fixtures";

function selectTab(model: RibbonModel, tabId: RibbonTabId): RibbonModel {
  if (!model.tabs.some((tab) => tab.id === tabId)) {
    throw new Error(`Ribbon interaction harness cannot select absent Tab ${tabId}.`);
  }
  return {
    ...model,
    tabs: model.tabs.map((tab) => ({ ...tab, selected: tab.id === tabId })),
  };
}

export interface RibbonM8IntegrationHarness {
  readonly dispose: () => void;
  readonly setReducedMotion: (value: boolean) => void;
  readonly setRoutingInFlight: (value: boolean) => void;
  readonly selectTab: (tabId: RibbonTabId) => void;
}

/** Mounts the real AppRibbon with only shell inputs exposed to browser evidence. */
export function mountRibbonM8IntegrationHarness(target: HTMLElement): RibbonM8IntegrationHarness {
  const [model, setModel] = createSignal<RibbonModel>(M6_RIBBON_FIXTURES.courseInstructor);
  const [routingInFlight, setRoutingInFlight] = createSignal(false);
  const [reducedMotion, setReducedMotion] = createSignal(false);
  const dispose = render(
    () => (
      <AppRibbon model={model()} routingInFlight={routingInFlight} reducedMotion={reducedMotion} />
    ),
    target,
  );
  return {
    dispose,
    setReducedMotion,
    setRoutingInFlight,
    selectTab: (tabId) => setModel((current) => selectTab(current, tabId)),
  };
}
