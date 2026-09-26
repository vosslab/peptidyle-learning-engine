// record_page_controls_harness.tsx - browser-only composition of the shared RecordPageControls API.

import { createSignal, type JSX } from "solid-js";
import { render } from "solid-js/web";

import "../../src/browser_environment";

import {
  RecordPageControls,
  type RecordPageSize,
} from "../../src/components/record_list/record_page_controls";

function RecordPageControlsHarness(): JSX.Element {
  const [hasPrevious, setHasPrevious] = createSignal(false);
  const [hasNext, setHasNext] = createSignal(true);
  const [loading, setLoading] = createSignal(false);
  const [pageSize, setPageSize] = createSignal<RecordPageSize>(50);
  const [lastAction, setLastAction] = createSignal("none");

  function movePrevious(): void {
    setHasPrevious(false);
    setHasNext(true);
    setLastAction("previous");
  }

  function moveNext(): void {
    setHasPrevious(true);
    setHasNext(false);
    setLastAction("next");
  }

  return (
    <main data-record-page-controls-harness>
      <h1>Bounded discovery pages</h1>
      <button type="button" onClick={() => setLoading((current) => !current)}>
        Toggle page loading
      </button>
      <RecordPageControls
        ariaLabel="Question Library pages"
        hasPrevious={hasPrevious()}
        hasNext={hasNext()}
        loading={loading()}
        onPrevious={movePrevious}
        onNext={moveNext}
        pageSize={pageSize()}
        onPageSizeChange={setPageSize}
      />
      <output
        data-record-page-controls-action={lastAction()}
        data-record-page-controls-size={pageSize()}
      >
        Last action: {lastAction()}; size: {pageSize()}
      </output>
      <RecordPageControls
        ariaLabel="Assessment entries pages"
        hasPrevious={false}
        hasNext={false}
        onPrevious={() => undefined}
        onNext={() => undefined}
      />
    </main>
  );
}

export function mountRecordPageControlsHarness(target: HTMLElement): void {
  render(() => <RecordPageControlsHarness />, target);
}
