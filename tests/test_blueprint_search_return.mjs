import assert from "node:assert/strict";
import test from "node:test";

import { blueprintDetailCollectionLink } from "../src/pages/blueprint_course_search_return_state.ts";

test("Blueprint detail collection follows current read access without a result snapshot", () => {
  assert.deepEqual(blueprintDetailCollectionLink("blueprint_course_owner"), {
    href: "/blueprint-courses",
    label: "Return to My Blueprint Courses",
  });
  assert.deepEqual(blueprintDetailCollectionLink("active_instructor"), {
    href: "/blueprint-courses/search/public",
    label: "Return to Public Blueprint Courses",
  });
});
