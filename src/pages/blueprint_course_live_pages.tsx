// blueprint_course_live_pages.tsx - production route composition for Blueprint Courses.

import { useParams } from "@solidjs/router";
import type { JSX } from "solid-js";

import { createQuestionLibraryRepository } from "../api/question_library_repository";
import { useApplicationApi } from "../api/application_api";
import {
  questionLibraryPickerRepository,
  questionLibraryPickerSources,
} from "../features/question_picker";
import { BlueprintCourseDetailRoutePage } from "./blueprint_course_detail_route_page";
import { BlueprintCoursesRoutePage } from "./blueprint_course_route_page";

interface BlueprintCourseRouteComposition {
  readonly client: ReturnType<typeof useApplicationApi>["client"];
  readonly pickerRepository: ReturnType<typeof questionLibraryPickerRepository>;
  readonly pickerSources: ReturnType<typeof questionLibraryPickerSources>;
}

/** Connects available Question Library sources for Blueprint Course Content editors. */
function useBlueprintCourseRouteComposition(): BlueprintCourseRouteComposition {
  const runtime = useApplicationApi();
  const questionLibrary = createQuestionLibraryRepository(runtime.client);
  const myQuestions = createQuestionLibraryRepository(runtime.client, "authoredByCurrentAccount");

  return {
    client: runtime.client,
    pickerRepository: questionLibraryPickerRepository(questionLibrary, myQuestions),
    pickerSources: questionLibraryPickerSources(true),
  };
}

/** `/blueprint-courses`: live Blueprint Course workspace. */
export function BlueprintCoursesLivePage(): JSX.Element {
  const composition = useBlueprintCourseRouteComposition();
  return (
    <BlueprintCoursesRoutePage
      client={composition.client}
      pickerRepository={composition.pickerRepository}
      pickerSources={composition.pickerSources}
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
      pickerSources={composition.pickerSources}
      blueprintCourseRef={params["blueprintCourseRef"] ?? ""}
    />
  );
}
