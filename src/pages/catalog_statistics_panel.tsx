// catalog_statistics_panel.tsx - safe anonymous evidence presentation for immutable catalog detail.

import { Show, type JSX } from "solid-js";

import type { CatalogStatisticsStatus } from "../../generated/api/CatalogStatisticsStatus";
import "./catalog_statistics_panel.css";

export interface CatalogStatisticsPanelProps {
  readonly statistics: CatalogStatisticsStatus;
}

type AvailableCatalogStatistics = Exclude<CatalogStatisticsStatus, "unavailable">["available"];
type DiscriminationEntry = {
  readonly value: NonNullable<AvailableCatalogStatistics["discriminationIndex"]>;
};

const wholeNumber = new Intl.NumberFormat("en-US");
const decimalNumber = new Intl.NumberFormat("en-US", { maximumFractionDigits: 2 });
const percentage = new Intl.NumberFormat("en-US", {
  style: "percent",
  maximumFractionDigits: 1,
});

function formatDuration(seconds: number): string {
  const roundedSeconds = Math.round(seconds);
  const minutes = Math.floor(roundedSeconds / 60);
  const remainingSeconds = roundedSeconds % 60;
  if (minutes === 0) return `${remainingSeconds} sec`;
  if (remainingSeconds === 0) return `${minutes} min`;
  return `${minutes} min ${remainingSeconds} sec`;
}

/** Renders only the generated client projection; disclosure policy stays server-owned. */
export function CatalogStatisticsPanel(props: CatalogStatisticsPanelProps): JSX.Element {
  const available = (): AvailableCatalogStatistics | undefined =>
    props.statistics === "unavailable" ? undefined : props.statistics.available;

  return (
    <Show
      when={available()}
      fallback={
        <section class="catalog-statistics-panel" aria-labelledby="insufficient-evidence-heading">
          <h2 id="insufficient-evidence-heading">Insufficient evidence</h2>
          <p>
            There is not enough anonymous learning evidence to display measures for this question.
          </p>
        </section>
      }
    >
      {(statistics) => <AvailableStatistics statistics={statistics()} />}
    </Show>
  );
}

function AvailableStatistics(props: {
  readonly statistics: AvailableCatalogStatistics;
}): JSX.Element {
  const discrimination = (): DiscriminationEntry | undefined => {
    const value = props.statistics.discriminationIndex;
    return value === undefined ? undefined : { value };
  };
  return (
    <section class="catalog-statistics-panel" aria-labelledby="anonymous-evidence-heading">
      <h2 id="anonymous-evidence-heading">Anonymous learning evidence</h2>
      <p class="catalog-statistics-introduction">
        These disclosed measures summarize an anonymous cohort for this published question.
      </p>
      <dl class="catalog-statistics-measures">
        <div>
          <dt>Cohort size</dt>
          <dd>{wholeNumber.format(props.statistics.cohortSize)} learners</dd>
        </div>
        <div>
          <dt>Difficulty (mean score)</dt>
          <dd>{percentage.format(props.statistics.difficultyIndex)}</dd>
        </div>
        <div>
          <dt>Mean submitted attempts</dt>
          <dd>{decimalNumber.format(props.statistics.attemptsMean)} attempts</dd>
        </div>
        <div>
          <dt>Estimated median duration</dt>
          <dd>{formatDuration(props.statistics.timeMedianSecondsEstimate)}</dd>
        </div>
        <Show when={discrimination()}>
          {(entry) => (
            <div>
              <dt>Discrimination</dt>
              <dd>{decimalNumber.format(entry().value)}</dd>
            </div>
          )}
        </Show>
      </dl>
    </section>
  );
}
