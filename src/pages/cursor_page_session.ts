// cursor_page_session.ts - strict, reusable cursor append state for visible page controls.

import type { CursorPage } from "../api/contracts";

export type CursorPageSessionError =
  | { readonly kind: "protocol"; readonly message: string }
  | { readonly kind: "transport"; readonly message: string };

export interface CursorPageSessionState<T> {
  readonly items: ReadonlyArray<T>;
  readonly nextCursor: string | null;
  readonly loading: boolean;
  readonly error: CursorPageSessionError | null;
}

export type CursorPageLoader<T> = (cursor: string) => Promise<CursorPage<T>>;
export type CursorPageItemKey<T> = (item: T) => string;
export type CursorPageSessionListener<T> = (state: CursorPageSessionState<T>) => void;

/**
 * Owns one bounded append-only cursor chain. A page is never fetched twice in
 * parallel, transport retries keep their opaque cursor, and malformed chains
 * stop instead of offering a misleading infinite retry control.
 */
export class CursorPageSession<T> {
  private current: CursorPageSessionState<T>;
  private inFlight: Promise<ReadonlyArray<T>> | undefined;
  private readonly acceptedRequestCursors = new Set<string>();

  public constructor(
    initialPage: CursorPage<T>,
    private readonly loadPage: CursorPageLoader<T>,
    private readonly itemKey: CursorPageItemKey<T>,
    private readonly onChange: CursorPageSessionListener<T> = () => undefined,
  ) {
    const items = this.dedupe(initialPage.items, []);
    this.current = { items, nextCursor: initialPage.nextCursor, loading: false, error: null };
  }

  public get state(): CursorPageSessionState<T> {
    return this.current;
  }

  public loadMore(): Promise<ReadonlyArray<T>> {
    if (this.inFlight !== undefined) {
      return this.inFlight;
    }
    const cursor = this.current.nextCursor;
    if (cursor === null) {
      return Promise.resolve([]);
    }
    const request = this.request(cursor);
    this.inFlight = request;
    void request.finally(() => {
      if (this.inFlight === request) {
        this.inFlight = undefined;
      }
    });
    return request;
  }

  public retry(): Promise<ReadonlyArray<T>> {
    if (this.current.error?.kind !== "transport") {
      return Promise.resolve([]);
    }
    return this.loadMore();
  }

  private async request(cursor: string): Promise<ReadonlyArray<T>> {
    this.setState({ ...this.current, loading: true, error: null });
    try {
      const page = await this.loadPage(cursor);
      const appended = this.dedupe(page.items, this.current.items);
      const protocolMessage = this.protocolMessage(cursor, page.nextCursor, appended.length);
      if (protocolMessage !== null) {
        this.setState({
          ...this.current,
          nextCursor: null,
          loading: false,
          error: { kind: "protocol", message: protocolMessage },
        });
        return [];
      }
      const items = [...this.current.items, ...appended];
      this.acceptedRequestCursors.add(cursor);
      this.setState({ items, nextCursor: page.nextCursor, loading: false, error: null });
      return appended;
    } catch {
      const message = "The next page could not be loaded.";
      this.setState({
        ...this.current,
        loading: false,
        error: { kind: "transport", message },
      });
      return [];
    }
  }

  private protocolMessage(
    cursor: string,
    nextCursor: string | null,
    appendedCount: number,
  ): string | null {
    if (nextCursor === null) {
      return null;
    }
    if (nextCursor === cursor || this.acceptedRequestCursors.has(nextCursor)) {
      return "The list returned a repeated page marker, so loading stopped safely.";
    }
    if (appendedCount === 0) {
      return "The list did not add new records, so loading stopped safely.";
    }
    return null;
  }

  private dedupe(items: ReadonlyArray<T>, priorItems: ReadonlyArray<T>): ReadonlyArray<T> {
    const keys = new Set(priorItems.map((item) => this.itemKey(item)));
    const unique: T[] = [];
    for (const item of items) {
      const key = this.itemKey(item);
      if (!keys.has(key)) {
        keys.add(key);
        unique.push(item);
      }
    }
    return unique;
  }

  private setState(next: CursorPageSessionState<T>): void {
    this.current = next;
    this.onChange(next);
  }
}
