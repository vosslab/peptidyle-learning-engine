// Public Blueprint detail collection destination.

export function blueprintDetailCollectionLink(readAccess: string | undefined): {
  readonly href: string;
  readonly label: string;
} {
  if (readAccess === "active_instructor") {
    return {
      href: "/blueprint-courses/search/public",
      label: "Return to Public Blueprint Courses",
    };
  }
  return { href: "/blueprint-courses", label: "Return to My Blueprint Courses" };
}
