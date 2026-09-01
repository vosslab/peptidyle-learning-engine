// blueprint_course_live_pages.tsx - production route composition for Blueprint Courses.

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
import { BlueprintCourseDetailRoutePage } from "./blueprint_course_detail_route_page";
import { BlueprintCoursesRoutePage } from "./blueprint_course_route_page";

interface BlueprintCourseRouteComposition {
  readonly client: ReturnType<typeof useApplicationApi>["client"];
  readonly pickerRepository: ReturnType<typeof createQuestionCurationRepository>["picker"];
  readonly pickerSources: () => ReturnType<typeof questionCurationPickerSources>;
}

/** Connects the live curation sources shared by Blueprint Course definition editors. */
function useBlueprintCourseRouteComposition(): BlueprintCourseRouteComposition {
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

/** `/blueprint-courses`: live Blueprint Course workspace. */
export function BlueprintCoursesLivePage(): JSX.Element {
  const composition = useBlueprintCourseRouteComposition();
  return (
    <BlueprintCoursesRoutePage
      client={composition.client}
      pickerRepository={composition.pickerRepository}
      pickerSources={composition.pickerSources()}
    />
  );
}

/** `/blueprint-courses/:blueprintCourseRef`: live Blueprint Course editor and inspection workspace. */
export function BlueprintCourseDetailLivePage(): JSX.Element {
  const composition = useBlueprintCourseRouteComposition();
  const params = useParams();
  return (
    <BlueprintCourseDetailRoutePage
      client={composition.client}
      pickerRepository={composition.pickerRepository}
      pickerSources={composition.pickerSources()}
      blueprintCourseRef={params["blueprintCourseRef"] ?? ""}
    />
  );
}
