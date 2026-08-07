// context.tsx - application-owned WASM startup, fallback, and consumer context.

import { createContext, createResource, Show, useContext, type JSX } from "solid-js";

import {
  loadWasmFacade,
  type CapabilityValidator,
  type FormatValidator,
  type TimerEvaluator,
  type WasmFacade,
} from "./index";

const WasmFacadeContext = createContext<WasmFacade>();

export interface WasmRuntimeProviderProps {
  readonly formatFallback: FormatValidator;
  readonly timerFallback: TimerEvaluator;
  readonly capabilityFallback: CapabilityValidator;
  readonly children: JSX.Element;
}

/** Initializes one shared module before any response widget is mounted. */
export function WasmRuntimeProvider(props: WasmRuntimeProviderProps): JSX.Element {
  const [facade] = createResource(() =>
    loadWasmFacade(props.formatFallback, props.timerFallback, props.capabilityFallback),
  );

  return (
    <Show
      when={facade()}
      fallback={
        <main class="startup" aria-busy="true">
          <p class="eyebrow">Preparing practice</p>
          <h1>Loading response tools...</h1>
        </main>
      }
    >
      {(readyFacade) => (
        <WasmFacadeContext.Provider value={readyFacade()}>
          <Show when={readyFacade().mode === "serverFallback"}>
            <div class="degraded-banner" role="status">
              Local response tools are unavailable. Format, timer, and assignment checks will use
              the server and may take a little longer.
            </div>
          </Show>
          {props.children}
        </WasmFacadeContext.Provider>
      )}
    </Show>
  );
}

/** Reads the app-owned key-free domain-tools facade. */
export function useWasmFacade(): WasmFacade {
  const facade = useContext(WasmFacadeContext);
  if (facade === undefined) {
    throw new Error("WasmRuntimeProvider is missing from the application root");
  }
  return facade;
}
