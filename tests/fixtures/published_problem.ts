// Stored browser boundary data for focused Node and Playwright tests.
//
// Question content belongs to browser_fixture.json. This module provides the
// existing typed module import without duplicating that content in executable
// source.

import publishedProblemFixtureData from "./published_problem/browser_fixture.json" with {
  type: "json",
};

export const publishedProblemFixture = publishedProblemFixtureData;
