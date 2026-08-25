// curriculum_live_pages.tsx - production route composition for reusable curricula.

import { useParams } from "@solidjs/router";
import { createSignal, onMount, type JSX } from "solid-js";

import { createCatalogRepository } from "../api/catalog_repository";
import { useApiRuntime } from "../api/runtime";
import { useSessionBootstrap } from "../auth/session_context";
import {
  problemCurationPickerSources,
  mayMutatePersonalCuration,
} from "../features/problem_curation/problem_curation_model";
import { createProblemCurationRepository } from "../features/problem_curation/problem_curation_repository";
import { CurriculumDetailRoutePage } from "./curriculum_detail_route_page";
import { CurriculumRoutePage } from "./curriculum_route_page";

interface CurriculumRouteComposition {
  readonly client: ReturnType<typeof useApiRuntime>["client"];
  readonly pickerRepository: ReturnType<typeof createProblemCurationRepository>["picker"];
  readonly pickerSources: () => ReturnType<typeof problemCurationPickerSources>;
}

/** Connects the live curation sources shared by curriculum definition editors. */
function useCurriculumRouteComposition(): CurriculumRouteComposition {
  const runtime = useApiRuntime();
  const session = useSessionBootstrap();
  const catalog = createCatalogRepository(runtime.client);
  const curation = createProblemCurationRepository(runtime.client, catalog);
  const [collections, setCollections] = createSignal<
    ReadonlyArray<
      import("../../generated/api/ProblemCollectionSummaryView").ProblemCollectionSummaryView
    >
  >([]);

  async function loadCollections(): Promise<void> {
    const page = await curation.curation.listCollections(null);
    setCollections(page.items);
  }

  onMount(() => void loadCollections());

  return {
    client: runtime.client,
    pickerRepository: curation.picker,
    pickerSources: (): ReturnType<typeof problemCurationPickerSources> => {
      const sessionState = session.state();
      return problemCurationPickerSources(
        collections(),
        mayMutatePersonalCuration(
          sessionState.kind === "authenticated" ? sessionState.session : undefined,
        ),
      );
    },
  };
}

/** `/curriculum`: live reusable-blueprint and Alpha-curriculum workspace. */
export function CurriculumLivePage(): JSX.Element {
  const composition = useCurriculumRouteComposition();
  return (
    <CurriculumRoutePage
      client={composition.client}
      pickerRepository={composition.pickerRepository}
      pickerSources={composition.pickerSources()}
    />
  );
}

/** `/curriculum/:curriculumRef`: live reusable-curriculum editor and inspection workspace. */
export function CurriculumDetailLivePage(): JSX.Element {
  const composition = useCurriculumRouteComposition();
  const params = useParams();
  return (
    <CurriculumDetailRoutePage
      client={composition.client}
      pickerRepository={composition.pickerRepository}
      pickerSources={composition.pickerSources()}
      curriculumRef={params["curriculumRef"] ?? ""}
    />
  );
}
