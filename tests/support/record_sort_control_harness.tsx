// record_sort_control_harness.tsx - browser-only composition of the shared RecordSortControl API.

import { createSignal, type JSX } from "solid-js";
import { render } from "solid-js/web";

import "../../src/browser_environment";

import { RecordSortControl } from "../../src/components/record_list/record_sort_control";

const SORT_OPTIONS = [
  { value: "title", label: "Title" },
  { value: "recent-publication", label: "Most recently published" },
] as const;

type SortValue = (typeof SORT_OPTIONS)[number]["value"];

function RecordSortControlHarness(): JSX.Element {
  const [value, setValue] = createSignal<SortValue>("title");
  const [changeCount, setChangeCount] = createSignal(0);

  function changeSort(nextValue: SortValue): void {
    setValue(nextValue);
    setChangeCount((count) => count + 1);
  }

  return (
    <main data-record-sort-control-harness>
      <h1>Result ordering</h1>
      <RecordSortControl
        label="Sort Question Library"
        options={SORT_OPTIONS}
        value={value()}
        onChange={changeSort}
      />
      <output
        data-record-sort-control-current={value()}
        data-record-sort-control-change-count={changeCount()}
      >
        Current sort: {value()}
      </output>
      <RecordSortControl
        label="Sort unavailable results"
        options={SORT_OPTIONS}
        value="title"
        disabled={true}
        onChange={changeSort}
      />
    </main>
  );
}

export function mountRecordSortControlHarness(target: HTMLElement): void {
  render(() => <RecordSortControlHarness />, target);
}
