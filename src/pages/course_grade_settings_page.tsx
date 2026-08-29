import { For, Show, createMemo, createSignal, onMount, type JSX } from "solid-js";

import type { CourseId } from "../../generated/api/CourseId";
import type { CourseGradeOutcomeView } from "../../generated/api/CourseGradeOutcomeView";
import type { CourseReference } from "../../generated/api/CourseReference";
import type { CourseGradeSchemeUpdateView } from "../../generated/api/CourseGradeSchemeUpdateView";
import type { CourseGradeSchemeView } from "../../generated/api/CourseGradeSchemeView";
import type { GradeCategoryId } from "../../generated/api/GradeCategoryId";
import { CourseGradeSchemeConflictError } from "../api/http_client";
import { useApiRuntime } from "../api/runtime";
import {
  courseRouteData,
  useCourseThemeRouteData,
} from "../features/course_appearance/course_theme_context";
import {
  canonicalizeAssignments,
  gradeSettingsErrors,
  percentToBasisPoints,
} from "./course_grade_settings_model";
import "./course_grade_settings_page.css";
import { formatPercentScore } from "../score_format";

type State = "loading" | "ready" | "saving" | "error";
interface CoursePageProps {
  readonly courseId: CourseId;
  readonly courseReference: CourseReference;
}

function draftFrom(view: CourseGradeSchemeView): CourseGradeSchemeUpdateView {
  return {
    scheme: structuredClone(view.scheme),
    assignments: view.assignments.map((item) => ({
      assignment: item.assignment,
      included: item.included,
      category: item.category,
      position: item.position,
    })),
  };
}

function newCategory(): CourseGradeSchemeUpdateView["scheme"]["categories"][number] {
  return {
    id: crypto.randomUUID(),
    title: "New category",
    position: 0,
    weightBasisPoints: 1,
    dropLowest: 0,
  };
}

function unavailableReasonCopy(outcome: CourseGradeOutcomeView): string {
  if (outcome.status !== "unavailable") return "";
  switch (outcome.reason) {
    case "noIncludedAssignments":
      return "No assignments are included";
    case "recalculating":
      return "Grades are being recalculated";
    case "failed":
      return "Grade calculation needs attention";
    case "emptyAfterDrop":
      return "No scores remain after applying drop rules";
    case "zeroPossiblePoints":
      return "Included assignments have no possible points";
  }
}

function GradeSettingsCoursePage(props: CoursePageProps): JSX.Element {
  const runtime = useApiRuntime();
  const [state, setState] = createSignal<State>("loading");
  const [draft, setDraft] = createSignal<CourseGradeSchemeUpdateView>();
  const [serverDraft, setServerDraft] = createSignal<CourseGradeSchemeUpdateView>();
  const [assignmentTitles, setAssignmentTitles] = createSignal(new Map<string, string>());
  const [revision, setRevision] = createSignal("");
  const [message, setMessage] = createSignal("");
  const [hasConflict, setHasConflict] = createSignal(false);
  const [errors, setErrors] = createSignal<ReadonlyArray<string>>([]);
  const [totals, setTotals] =
    createSignal<Awaited<ReturnType<typeof runtime.client.getCourseGradebookTotals>>>();
  let errorSummary: HTMLDivElement | undefined;
  const busy = createMemo(() => state() === "saving");
  const weight = createMemo(
    () => draft()?.scheme.categories.reduce((sum, item) => sum + item.weightBasisPoints, 0) ?? 0,
  );

  function replaceDraft(next: CourseGradeSchemeUpdateView): void {
    setDraft(canonicalizeAssignments(next));
  }
  function checkDraft(current: CourseGradeSchemeUpdateView): boolean {
    const nextErrors = gradeSettingsErrors(current);
    setErrors(nextErrors);
    if (nextErrors.length > 0) queueMicrotask(() => errorSummary?.focus());
    return nextErrors.length === 0;
  }
  async function load(showReloadMessage = false): Promise<void> {
    setState("loading");
    setErrors([]);
    setHasConflict(false);
    try {
      const [view, projectedTotals] = await Promise.all([
        runtime.client.getCourseGradeScheme(props.courseId),
        runtime.client.getCourseGradebookTotals(props.courseId),
      ]);
      const next = draftFrom(view);
      replaceDraft(next);
      setServerDraft(next);
      setAssignmentTitles(new Map(view.assignments.map((item) => [item.assignment, item.title])));
      setRevision(view.revision);
      setTotals(projectedTotals);
      setMessage(showReloadMessage ? "Latest server settings loaded." : "");
      setState("ready");
    } catch {
      setState("error");
      setMessage("Grade settings could not load. Try again.");
    }
  }
  function chooseMode(mode: "totalPoints" | "weightedCategories"): void {
    const current = draft();
    if (current === undefined) return;
    const categories =
      mode === "totalPoints"
        ? []
        : current.scheme.categories.length === 0
          ? [{ ...newCategory(), title: "Course work", weightBasisPoints: 10_000 }]
          : current.scheme.categories;
    replaceDraft({
      ...current,
      scheme: { ...current.scheme, mode, categories },
      assignments: current.assignments.map((item) => ({
        ...item,
        category: mode === "totalPoints" ? null : item.category,
        position: mode === "totalPoints" ? null : item.position,
      })),
    });
  }
  function addCategory(): void {
    const current = draft();
    if (current === undefined) return;
    const category = newCategory();
    category.position = current.scheme.categories.length;
    replaceDraft({
      ...current,
      scheme: { ...current.scheme, categories: [...current.scheme.categories, category] },
    });
  }
  function removeCategory(categoryId: GradeCategoryId): void {
    const current = draft();
    if (current === undefined) return;
    const categories = current.scheme.categories
      .filter((item) => item.id !== categoryId)
      .map((item, index) => ({ ...item, position: index }));
    replaceDraft({
      ...current,
      scheme: { ...current.scheme, categories },
      assignments: current.assignments.map((item) =>
        item.category === categoryId ? { ...item, category: null, position: null } : item,
      ),
    });
  }
  function moveCategory(categoryId: GradeCategoryId, direction: -1 | 1): void {
    const current = draft();
    if (current === undefined) return;
    const index = current.scheme.categories.findIndex((item) => item.id === categoryId);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= current.scheme.categories.length) return;
    const categories = [...current.scheme.categories];
    const [item] = categories.splice(index, 1);
    categories.splice(target, 0, item!);
    replaceDraft({
      ...current,
      scheme: {
        ...current.scheme,
        categories: categories.map((item, position) => ({ ...item, position })),
      },
    });
  }
  function editCategory(
    categoryId: GradeCategoryId,
    field: "title" | "weightBasisPoints" | "dropLowest",
    value: string,
  ): void {
    const current = draft();
    if (current === undefined) return;
    const numeric = field === "weightBasisPoints" ? percentToBasisPoints(value) : Number(value);
    if (field !== "title" && (!Number.isSafeInteger(numeric) || numeric! < 0 || numeric! > 10_000))
      return;
    replaceDraft({
      ...current,
      scheme: {
        ...current.scheme,
        categories: current.scheme.categories.map((item) =>
          item.id === categoryId
            ? { ...item, [field]: field === "title" ? value : numeric! }
            : item,
        ),
      },
    });
  }
  function editAssignment(
    assignmentId: CourseGradeSchemeUpdateView["assignments"][number]["assignment"],
    included: boolean,
    category: GradeCategoryId | null,
  ): void {
    const current = draft();
    if (current === undefined) return;
    replaceDraft({
      ...current,
      assignments: current.assignments.map((item) =>
        item.assignment === assignmentId
          ? {
              ...item,
              included,
              category,
              position: category === null ? null : (item.position ?? 0),
            }
          : item,
      ),
    });
  }
  function selectedCategory(value: string): GradeCategoryId | null {
    const current = draft();
    return current?.scheme.categories.find((item) => item.id === value)?.id ?? null;
  }
  function addBand(): void {
    const current = draft();
    if (current === undefined) return;
    replaceDraft({
      ...current,
      scheme: {
        ...current.scheme,
        letterBands: [...current.scheme.letterBands, { label: "Letter", minimumBasisPoints: 0 }],
      },
    });
  }
  function editBand(index: number, field: "label" | "minimumBasisPoints", value: string): void {
    const current = draft();
    if (current === undefined) return;
    const points = field === "minimumBasisPoints" ? percentToBasisPoints(value) : undefined;
    if (field === "minimumBasisPoints" && points === undefined) return;
    replaceDraft({
      ...current,
      scheme: {
        ...current.scheme,
        letterBands: current.scheme.letterBands.map((item, itemIndex) =>
          itemIndex === index ? { ...item, [field]: field === "label" ? value : points! } : item,
        ),
      },
    });
  }
  async function save(): Promise<void> {
    const current = draft();
    if (current === undefined || !checkDraft(current)) return;
    setState("saving");
    try {
      const saved = await runtime.client.saveCourseGradeScheme(props.courseId, current, revision());
      const next = draftFrom(saved);
      replaceDraft(next);
      setServerDraft(next);
      setAssignmentTitles(new Map(saved.assignments.map((item) => [item.assignment, item.title])));
      setRevision(saved.revision);
      setState("ready");
      setHasConflict(false);
      setMessage("Grade settings saved.");
      try {
        setTotals(await runtime.client.getCourseGradebookTotals(props.courseId));
      } catch {
        setMessage(
          "Grade settings saved, but totals could not refresh. Use Reload current settings to try again.",
        );
      }
    } catch (error: unknown) {
      setState("ready");
      if (error instanceof CourseGradeSchemeConflictError) {
        setHasConflict(true);
        try {
          const latest = await runtime.client.getCourseGradeScheme(props.courseId);
          setServerDraft(draftFrom(latest));
          setAssignmentTitles(
            new Map(latest.assignments.map((item) => [item.assignment, item.title])),
          );
          setRevision(latest.revision);
          setMessage(
            "The settings changed elsewhere. Your draft is preserved; adopt latest or retry.",
          );
        } catch {
          setMessage(
            "The settings changed elsewhere. Your draft is preserved; reload latest settings.",
          );
        }
      } else {
        setHasConflict(false);
        setMessage("Grade settings could not be saved.");
      }
    }
  }
  function adoptCurrent(): void {
    const current = serverDraft();
    if (current !== undefined) {
      replaceDraft(current);
      setErrors([]);
      setHasConflict(false);
      setMessage("Latest server settings adopted. You can edit and save again.");
    }
  }
  async function exportCsv(): Promise<void> {
    try {
      const file = await runtime.client.createCourseGradeExport(props.courseId);
      const url = URL.createObjectURL(file.csv);
      const link = document.createElement("a");
      link.href = url;
      link.download = file.filename;
      document.body.append(link);
      link.click();
      link.remove();
      setTimeout(() => URL.revokeObjectURL(url), 0);
      setMessage("Audited course export is ready.");
    } catch {
      setMessage("Course export could not be created.");
    }
  }
  onMount(() => {
    void load();
  });
  return (
    <section class="page course-grade-settings" data-route-surface="courseGradeSettings">
      <h1>Course grade settings</h1>
      <p id="grade-settings-status" role="status" aria-live="polite">
        {message()}
      </p>
      <Show when={state() === "loading"}>
        <p>Loading grade settings...</p>
      </Show>
      <Show when={state() === "error"}>
        <button
          type="button"
          onClick={() => {
            void load();
          }}
        >
          Retry loading grade settings
        </button>
      </Show>
      <Show when={draft()}>
        {(current) => (
          <>
            <Show when={errors().length > 0}>
              <div
                class="course-grade-errors"
                role="alert"
                tabindex="-1"
                ref={(element) => {
                  errorSummary = element;
                }}
              >
                <h2>Fix these grade settings</h2>
                <ul>
                  <For each={errors()}>
                    {(error) => (
                      <li>
                        <a href="#grade-settings-form">{error}</a>
                      </li>
                    )}
                  </For>
                </ul>
              </div>
            </Show>
            <form
              id="grade-settings-form"
              onSubmit={(event) => {
                event.preventDefault();
                void save();
              }}
            >
              <fieldset disabled={busy()}>
                <legend>Aggregation method</legend>
                <label>
                  <input
                    type="radio"
                    name="grade-mode"
                    checked={current().scheme.mode === "totalPoints"}
                    onChange={() => chooseMode("totalPoints")}
                  />{" "}
                  Total points
                </label>
                <label>
                  <input
                    type="radio"
                    name="grade-mode"
                    checked={current().scheme.mode === "weightedCategories"}
                    onChange={() => chooseMode("weightedCategories")}
                  />{" "}
                  Weighted categories
                </label>
              </fieldset>
              <Show when={current().scheme.mode === "weightedCategories"}>
                <section>
                  <h2>Weighted categories</h2>
                  <p>Weight total: {(weight() / 100).toFixed(2)}% of 100.00%.</p>
                  <For each={current().scheme.categories}>
                    {(category) => (
                      <fieldset>
                        <legend>{category.title}</legend>
                        <label>
                          Title{" "}
                          <input
                            value={category.title}
                            onInput={(event) =>
                              editCategory(category.id, "title", event.currentTarget.value)
                            }
                          />
                        </label>
                        <label>
                          Weight (%){" "}
                          <input
                            type="number"
                            inputmode="decimal"
                            min="0.01"
                            max="100"
                            step="0.01"
                            required
                            value={(category.weightBasisPoints / 100).toFixed(2)}
                            onInput={(event) =>
                              editCategory(
                                category.id,
                                "weightBasisPoints",
                                event.currentTarget.value,
                              )
                            }
                          />
                        </label>
                        <label>
                          Drop lowest{" "}
                          <input
                            type="number"
                            min="0"
                            max="10000"
                            step="1"
                            required
                            value={category.dropLowest}
                            onInput={(event) =>
                              editCategory(category.id, "dropLowest", event.currentTarget.value)
                            }
                          />
                        </label>
                        <button
                          type="button"
                          aria-label={`Move ${category.title} earlier`}
                          onClick={() => moveCategory(category.id, -1)}
                        >
                          Move earlier
                        </button>
                        <button
                          type="button"
                          aria-label={`Move ${category.title} later`}
                          onClick={() => moveCategory(category.id, 1)}
                        >
                          Move later
                        </button>
                        <button
                          type="button"
                          aria-label={`Remove ${category.title} category`}
                          onClick={() => removeCategory(category.id)}
                        >
                          Remove category
                        </button>
                      </fieldset>
                    )}
                  </For>
                  <button type="button" onClick={addCategory}>
                    Add category
                  </button>
                </section>
              </Show>
              <section>
                <h2>Assignments</h2>
                <For each={current().assignments}>
                  {(assignment) => (
                    <fieldset>
                      <legend>
                        {assignmentTitles().get(assignment.assignment) ?? "Assignment"}
                      </legend>
                      <label>
                        <input
                          type="checkbox"
                          checked={assignment.included}
                          onChange={(event) =>
                            editAssignment(
                              assignment.assignment,
                              event.currentTarget.checked,
                              assignment.category,
                            )
                          }
                        />{" "}
                        Include assignment
                      </label>
                      <Show when={current().scheme.mode === "weightedCategories"}>
                        <label>
                          Category{" "}
                          <select
                            value={assignment.category ?? ""}
                            onChange={(event) =>
                              editAssignment(
                                assignment.assignment,
                                assignment.included,
                                selectedCategory(event.currentTarget.value),
                              )
                            }
                          >
                            <option value="">Select a category</option>
                            <For each={current().scheme.categories}>
                              {(category) => <option value={category.id}>{category.title}</option>}
                            </For>
                          </select>
                        </label>
                      </Show>
                    </fieldset>
                  )}
                </For>
              </section>
              <section>
                <h2>Letter bands (optional)</h2>
                <For each={current().scheme.letterBands}>
                  {(band, index) => (
                    <fieldset>
                      <legend>Letter band {index() + 1}</legend>
                      <label>
                        Label{" "}
                        <input
                          value={band.label}
                          onInput={(event) => editBand(index(), "label", event.currentTarget.value)}
                        />
                      </label>
                      <label>
                        Minimum (%){" "}
                        <input
                          type="number"
                          inputmode="decimal"
                          min="0"
                          max="100"
                          step="0.01"
                          required
                          value={(band.minimumBasisPoints / 100).toFixed(2)}
                          onInput={(event) =>
                            editBand(index(), "minimumBasisPoints", event.currentTarget.value)
                          }
                        />
                      </label>
                      <button
                        type="button"
                        onClick={() =>
                          replaceDraft({
                            ...current(),
                            scheme: {
                              ...current().scheme,
                              letterBands: current().scheme.letterBands.filter(
                                (_item, itemIndex) => itemIndex !== index(),
                              ),
                            },
                          })
                        }
                      >
                        Remove band
                      </button>
                    </fieldset>
                  )}
                </For>
                <button type="button" onClick={addBand}>
                  Add letter band
                </button>
              </section>
              <div>
                <button type="submit" disabled={busy()}>
                  Save grade settings
                </button>
                <button
                  type="button"
                  disabled={busy()}
                  onClick={() => {
                    void load(true);
                  }}
                >
                  Reload current settings
                </button>
                <Show when={hasConflict()}>
                  <button type="button" onClick={adoptCurrent}>
                    Adopt latest settings
                  </button>
                </Show>
              </div>
            </form>
            <section>
              <h2>Server-projected course totals</h2>
              <p>
                Totals are calculated by the server. This page does not calculate student grades.
              </p>
              <button
                type="button"
                onClick={() => {
                  void exportCsv();
                }}
              >
                Export audited course grades CSV
              </button>
              <Show
                when={totals()}
                fallback={<p class="empty-state">No course totals are available yet.</p>}
              >
                {(value) => (
                  <Show
                    when={value().rows.length > 0}
                    fallback={
                      <p class="empty-state">No enrolled students have a course total yet.</p>
                    }
                  >
                    <table>
                      <caption>Current server-projected course totals</caption>
                      <thead>
                        <tr>
                          <th>Student</th>
                          <th>Course total</th>
                          <th>Letter</th>
                        </tr>
                      </thead>
                      <tbody>
                        <For each={value().rows}>
                          {(row) => (
                            <tr>
                              <td>{row.displayName}</td>
                              <Show
                                when={row.outcome.status === "available"}
                                fallback={
                                  <>
                                    <td>Unavailable</td>
                                    <td>{unavailableReasonCopy(row.outcome)}</td>
                                  </>
                                }
                              >
                                <td>
                                  {row.outcome.status === "available"
                                    ? formatPercentScore(row.outcome.score)
                                    : ""}
                                </td>
                                <td>
                                  {row.outcome.status === "available"
                                    ? (row.outcome.letter ?? "No letter band")
                                    : ""}
                                </td>
                              </Show>
                            </tr>
                          )}
                        </For>
                      </tbody>
                    </table>
                  </Show>
                )}
              </Show>
            </section>
          </>
        )}
      </Show>
    </section>
  );
}

export function CourseGradeSettingsPage(): JSX.Element {
  const route = useCourseThemeRouteData();
  const course = route?.kind === "course" ? courseRouteData(route).summary : undefined;
  return (
    <Show
      when={course}
      keyed
      fallback={
        <section class="page course-grade-settings">
          <h1>Course grade settings unavailable</h1>
        </section>
      }
    >
      {(value) => <GradeSettingsCoursePage courseId={value.id} courseReference={value.reference} />}
    </Show>
  );
}
