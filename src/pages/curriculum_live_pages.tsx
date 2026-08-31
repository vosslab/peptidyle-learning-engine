// curriculum_live_pages.tsx - production route composition for reusable curricula.

import { useParams } from "@solidjs/router";
import { createSignal, onMount, type JSX } from "solid-js";

import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { useApplicationApi } from "../api/application_api";
import { useSessionBootstrap } from "../auth/session_context";
import {
  questionCurationPickerSources,
  mayMutatePersonalCuration,
} from "../features/question_curation/question_curation_model";
import { createQuestionCurationRepository } from "../features/question_curation/question_curation_repository";
import { CurriculumDetailRoutePage } from "./curriculum_detail_route_page";
import { CurriculumRoutePage } from "./curriculum_route_page";

interface CurriculumRouteComposition {
  readonly client: ReturnType<typeof useApplicationApi>["client"];
  readonly pickerRepository: ReturnType<typeof createQuestionCurationRepository>["picker"];
  readonly pickerSources: () => ReturnType<typeof questionCurationPickerSources>;
}

/** Connects the live curation sources shared by curriculum definition editors. */
function useCurriculumRouteComposition(): CurriculumRouteComposition {
  const runtime = useApplicationApi();
  const session = useSessionBootstrap();
  const questionLibrary = createQuestionLibraryRepository(runtime.client);
  const curation = createQuestionCurationRepository(runtime.client, questionLibrary);
  const [folders, setFolders] = createSignal<
    ReadonlyArray<import("../../generated/api/QuestionFolderSummaryView").QuestionFolderSummaryView>
  >([]);

  async function loadFolders(): Promise<void> {
    const page = await curation.curation.listFolders(null);
    setFolders(page.items);
  }

  onMount(() => void loadFolders());

  return {
    client: runtime.client,
    pickerRepository: curation.picker,
    pickerSources: (): ReturnType<typeof questionCurationPickerSources> => {
      const sessionState = session.state();
      return questionCurationPickerSources(
        folders(),
        mayMutatePersonalCuration(
          sessionState.kind === "authenticated" ? sessionState.session : undefined,
        ),
      );
    },
  };
}

/** `/curriculum`: live Blueprint Course workspace. */
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
