// library_route_page.tsx - runtime composition for the production catalog surface.

import type { JSX } from "solid-js";

import { createCatalogRepository } from "../api/catalog_repository";
import { useApiRuntime } from "../api/runtime";
import { LibraryPage } from "./library_page";

export function LibraryRoutePage(): JSX.Element {
  const runtime = useApiRuntime();
  return <LibraryPage repository={createCatalogRepository(runtime.client)} />;
}
