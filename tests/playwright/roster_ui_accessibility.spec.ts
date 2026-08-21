// Built roster and account-security behavior. Selectors bind to visible labels and dialog roles.

import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

import { publishedProblemFixture } from "../../generated/fixtures/published_problem";
import type { CourseRosterMember } from "../../src/api/enrollment";

const COURSE = "C-1";

async function navigateWithinSpa(page: Page, pathname: string): Promise<void> {
  await page.evaluate((nextPath) => {
    history.pushState({}, "", nextPath);
    window.dispatchEvent(new PopStateEvent("popstate"));
  }, pathname);
}

async function expectNoBlockingAxeViolations(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).include("main").analyze();
  const blocking = results.violations.filter(
    (violation) => violation.impact === "critical" || violation.impact === "serious",
  );
  expect(blocking).toEqual([]);
}

function json(
  value: unknown,
  headers: Record<string, string> = {},
): { body: string; headers: Record<string, string>; contentType: string } {
  return {
    body: JSON.stringify(value),
    contentType: "application/json",
    headers: { "cache-control": "no-store", ...headers },
  };
}

async function useInstructorApi(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(window, "__PLE_USE_MOCK_API__", {
      configurable: false,
      get: () => false,
    });
  });
  const course = { ...publishedProblemFixture.course, role: "instructor" as const };
  const member: CourseRosterMember = {
    memberId: "0198e000-0000-7000-8000-000000000602",
    displayName: "Fixture Student",
    rosterEmail: "student@mail.roosevelt.edu",
    rosterId: "900123456",
    role: "student" as const,
    status: "active" as const,
  };
  const invitation = {
    invitationId: "0198e000-0000-7000-8000-000000000601",
    email: "student@example.edu",
    rosterId: "900123456",
    status: "pending" as const,
    expiresAt: 1_755_411_600_000,
  };
  let pendingInvitations: ReadonlyArray<typeof invitation> = [];
  let members: ReadonlyArray<typeof member> = [member];
  let passkeys = [
    {
      id: "0198e000-0000-7000-8000-000000000603",
      label: "Fixture laptop",
      createdAtMillis: 1_754_806_800_000,
      lastUsedAtMillis: null,
    },
  ];
  let revision = 1;
  const roster = (): unknown => ({
    rosterMode: "emailEnrollment",
    members,
    pendingInvitations,
    allowedEmailDomains: [{ domain: "mail.roosevelt.edu", includeSubdomains: false }],
    signupPosture: "invitationOnly",
    nextCursor: null,
    rosterRevision: revision,
  });
  await page.route("**/api/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path === "/api/auth/session") {
      return await route.fulfill(
        json({
          authenticated: true,
          tenant: course.tenant,
          user: {
            id: publishedProblemFixture.enrollment.user,
            displayName: "Instructor",
            roles: ["instructor"],
          },
        }),
      );
    }
    if (path === "/api/auth/account/presentation")
      return await route.fulfill(json({ contrast: "standard" }));
    if (path === "/api/courses")
      return await route.fulfill(json({ items: [course], nextCursor: null }));
    if (path === "/api/navigation/C-1")
      return await route.fulfill(json({ kind: "course", courseId: course.id }));
    if (path === `/api/courses/${course.id}`) return await route.fulfill(json(course));
    if (path === `/api/courses/${course.id}/appearance`)
      return await route.fulfill(
        json({ theme: "grass", revision: "1", banner: null }, { etag: '"1"' }),
      );
    if (path === `/api/courses/${course.id}/assignments`) {
      const { id, reference, title } = publishedProblemFixture.assignment;
      return await route.fulfill(json({ items: [{ id, reference, title }], nextCursor: null }));
    }
    if (path === `/api/courses/${course.id}/roster`) return await route.fulfill(json(roster()));
    if (path === `/api/courses/${course.id}/invitations` && request.method() === "POST") {
      pendingInvitations = [invitation];
      revision += 1;
      return await route.fulfill(
        json(
          {
            invitation,
            redemptionPath: `/course-invitations/redeem#token=${"A".repeat(43)}`,
            emailDelivery: "queued",
          },
          { etag: `"${revision}"` },
        ),
      );
    }
    if (
      path === `/api/courses/${course.id}/invitations/${invitation.invitationId}` &&
      request.method() === "DELETE"
    ) {
      pendingInvitations = [];
      revision += 1;
      return await route.fulfill(json({ rosterRevision: revision }, { etag: `"${revision}"` }));
    }
    if (
      path === `/api/courses/${course.id}/members/${member.memberId}` &&
      request.method() === "DELETE"
    ) {
      members = [{ ...member, status: "revoked" }];
      revision += 1;
      return await route.fulfill(json({ rosterRevision: revision }, { etag: `"${revision}"` }));
    }
    if (path === `/api/courses/${course.id}/roster-imports/preview`) {
      return await route.fulfill(
        json(
          {
            importId: "0198e000-0000-7000-8000-000000000604",
            state: "preview",
            expiresAt: 1_754_810_400_000,
            rosterRevision: revision,
            importRevision: 1,
            rows: [
              {
                rowNumber: 2,
                email: "new.student@mail.roosevelt.edu",
                rosterId: "900123457",
                status: "readyToInvite",
                reason: "ready",
              },
            ],
          },
          { etag: '"1"' },
        ),
      );
    }
    if (path.includes("/roster-imports/") && path.endsWith("/commit")) {
      return await route.fulfill(
        json(
          {
            importId: "0198e000-0000-7000-8000-000000000604",
            importRevision: 2,
            rosterRevision: revision + 1,
            invitationsCreated: 1,
            delivery: [{ rowNumber: 2, outcome: "needsAttention" }],
          },
          { etag: '"2"' },
        ),
      );
    }
    if (path === "/api/auth/passkeys") {
      if (request.method() === "GET") return await route.fulfill(json(passkeys));
      return await route.fulfill({ status: 204 });
    }
    if (path.startsWith("/api/auth/passkeys/") && request.method() === "DELETE") {
      passkeys = [];
      return await route.fulfill({ status: 204, headers: { "cache-control": "no-store" } });
    }
    return await route.fulfill({ status: 404, body: "not found" });
  });
}

test("roster import gives instructors a safe correction path and keyboard-ready selection", async ({
  page,
}) => {
  await useInstructorApi(page);
  await page.goto("/");
  await navigateWithinSpa(page, `/instructor/courses/${COURSE}/students`);

  const template = page.getByRole("button", { name: "Download CSV template" });
  await expect(template).toBeVisible();
  const downloadPromise = page.waitForEvent("download");
  await template.click();
  const download = await downloadPromise;
  expect(download.suggestedFilename()).toBe("ple-roster-import-template.csv");

  await page.getByLabel("Roster CSV").setInputFiles({
    name: "roster.csv",
    mimeType: "text/csv",
    buffer: Buffer.from("email,roster_id\nstudent@example.edu,900123456\n"),
  });
  await page.getByRole("button", { name: "Preview roster" }).click();
  const select = page.getByRole("checkbox", { name: "Invite CSV row 2" });
  await expect(select).toHaveAttribute("aria-checked", "true");
  await select.focus();
  await page.keyboard.press("Space");
  await expect(select).toHaveAttribute("aria-checked", "false");
  await page.keyboard.press("Space");
  await page.getByRole("button", { name: "Send selected invitations" }).click();
  await expect(page.getByRole("heading", { name: "Bulk invitation status" })).toBeVisible();
  await expect(page.getByText("Needs attention", { exact: true })).toBeVisible();
  await expect(
    page.getByText("Correct the email or roster ID, then re-upload", { exact: true }),
  ).toHaveCount(0);
  await expectNoBlockingAxeViolations(page);
});

test("destructive roster and passkey actions explain consequences and recover focus", async ({
  page,
}) => {
  await useInstructorApi(page);
  await page.goto("/");
  await navigateWithinSpa(page, `/instructor/courses/${COURSE}/students`);

  await page.getByLabel("Institutional email").fill("student@example.edu");
  await page.getByLabel("Institutional student ID").fill("900123456");
  await page.getByRole("button", { name: "Create invitation" }).click();
  const cancelInvitation = page.getByRole("button", { name: "Cancel invitation" });
  await cancelInvitation.click();
  const inviteDialog = page.getByRole("dialog", { name: "Cancel this invitation?" });
  await expect(inviteDialog).toContainText("can no longer claim");
  await expect(inviteDialog.getByRole("button", { name: "Cancel invitation" })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(cancelInvitation).toBeFocused();

  await page.getByRole("button", { name: "Revoke course access" }).click();
  const revokeDialog = page.getByRole("dialog", {
    name: "Revoke this student's course access?",
  });
  await expect(revokeDialog).toContainText("immediately loses course access");
  await revokeDialog.getByRole("button", { name: "Keep it" }).click();
  await expect(page.getByRole("button", { name: "Revoke course access" })).toBeFocused();

  await navigateWithinSpa(page, "/account/security");
  const removePasskey = page.getByRole("button", { name: "Remove passkey" });
  await removePasskey.click();
  const passkeyDialog = page.getByRole("dialog", { name: "Remove this passkey?" });
  await expect(passkeyDialog).toContainText("verified email remains available");
  await passkeyDialog.getByRole("button", { name: "Keep passkey" }).click();
  await expect(removePasskey).toBeFocused();
  await removePasskey.click();
  await page
    .getByRole("dialog", { name: "Remove this passkey?" })
    .getByRole("button", {
      name: "Remove passkey",
    })
    .click();
  await expect(page.getByRole("heading", { name: "No passkeys added" })).toBeVisible();
  await expect(page.getByRole("link", { name: "Add a passkey" })).toBeVisible();
  await expectNoBlockingAxeViolations(page);
});

test("roster import remains contained at compact and wide viewports", async ({ page }) => {
  await useInstructorApi(page);
  await page.goto("/");
  await navigateWithinSpa(page, `/instructor/courses/${COURSE}/students`);
  await page.getByLabel("Roster CSV").setInputFiles({
    name: "roster.csv",
    mimeType: "text/csv",
    buffer: Buffer.from("email,roster_id\nstudent@example.edu,900123456\n"),
  });
  await page.getByRole("button", { name: "Preview roster" }).click();
  await page.getByLabel("Institutional email").fill("student@example.edu");
  await page.getByLabel("Institutional student ID").fill("900123456");
  await page.getByRole("button", { name: "Create invitation" }).click();
  await page.getByRole("button", { name: "Cancel invitation" }).click();
  const dialog = page.getByRole("dialog", { name: "Cancel this invitation?" });
  for (const width of [320, 480, 768, 1920]) {
    await page.setViewportSize({ width, height: 800 });
    await expect(page.getByRole("table").last()).toBeVisible();
    await expect(page.getByRole("checkbox", { name: "Invite CSV row 2" })).toBeVisible();
    await expect(dialog).toBeVisible();
    const overflow = await page.evaluate(
      () => document.documentElement.scrollWidth > window.innerWidth,
    );
    expect(overflow).toBe(false);
  }
});
