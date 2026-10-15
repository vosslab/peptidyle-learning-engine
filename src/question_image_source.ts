// Content-relative image references identify supplied files, never network locations.

export function isQuestionSourceRelativeName(value: string): boolean {
  return (
    value.trim().length > 0 &&
    new TextEncoder().encode(value).length <= 4096 &&
    !/[\\:?#]/u.test(value) &&
    !Array.from(value).some((character) => {
      const code = character.codePointAt(0)!;
      return code < 32 || code === 127;
    }) &&
    !value.startsWith("/") &&
    !value.split("/").some((part) => part === "" || part === "." || part === "..")
  );
}
