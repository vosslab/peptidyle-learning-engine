// Shared account-session course choice used after every browser authentication path.

import { For, type JSX } from "solid-js";

import type { AccountCourse } from "../api/enrollment";

export interface AccountCoursePickerProps {
  readonly courses: ReadonlyArray<AccountCourse>;
  readonly select: (course: AccountCourse) => Promise<void>;
  readonly busy: boolean;
  readonly headingRef?: (element: HTMLHeadingElement) => void;
}

/** Keeps every account authentication path on the same ordinary course-session transition. */
export function AccountCoursePicker(props: AccountCoursePickerProps): JSX.Element {
  return (
    <section class="auth-panel" aria-labelledby="account-courses-heading">
      <h2 id="account-courses-heading" tabindex="-1" ref={props.headingRef}>
        Choose your course
      </h2>
      <p>Your PLE account can belong to courses from different instructors.</p>
      <div class="account-course-list">
        <For each={props.courses}>
          {(course) => (
            <button
              class="quiet-action account-course-action"
              type="button"
              disabled={props.busy}
              onClick={() => void props.select(course)}
            >
              <span>{course.title}</span>
              <small>{course.role}</small>
            </button>
          )}
        </For>
      </div>
    </section>
  );
}
