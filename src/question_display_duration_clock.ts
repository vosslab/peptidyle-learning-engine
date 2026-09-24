// Monotonic accumulator for approximate milliseconds shown with one Question.

export class QuestionDisplayDurationClock {
  #elapsedMilliseconds: number;
  #startedAtMilliseconds: number | null = null;

  constructor(savedMilliseconds: number | null) {
    if (
      savedMilliseconds !== null &&
      (!Number.isSafeInteger(savedMilliseconds) || savedMilliseconds < 0)
    ) {
      throw new RangeError("Saved Question display duration must be a nonnegative safe integer.");
    }
    this.#elapsedMilliseconds = savedMilliseconds ?? 0;
  }

  resume(nowMilliseconds: number): void {
    this.#requireMonotonicInstant(nowMilliseconds);
    if (this.#startedAtMilliseconds === null) this.#startedAtMilliseconds = nowMilliseconds;
  }

  pause(nowMilliseconds: number): number {
    const total = this.snapshot(nowMilliseconds);
    this.#elapsedMilliseconds = total;
    this.#startedAtMilliseconds = null;
    return total;
  }

  checkpoint(nowMilliseconds: number): number {
    const total = this.snapshot(nowMilliseconds);
    this.#elapsedMilliseconds = total;
    if (this.#startedAtMilliseconds !== null) this.#startedAtMilliseconds = nowMilliseconds;
    return total;
  }

  snapshot(nowMilliseconds: number): number {
    this.#requireMonotonicInstant(nowMilliseconds);
    if (this.#startedAtMilliseconds === null) return this.#elapsedMilliseconds;
    const additional = Math.floor(Math.max(0, nowMilliseconds - this.#startedAtMilliseconds));
    const total = this.#elapsedMilliseconds + additional;
    if (!Number.isSafeInteger(total)) {
      throw new RangeError("Question display duration exceeds the supported safe integer range.");
    }
    return total;
  }

  observeStoredMilliseconds(storedMilliseconds: number): void {
    if (!Number.isSafeInteger(storedMilliseconds) || storedMilliseconds < 0) {
      throw new RangeError("Stored Question display duration must be a nonnegative safe integer.");
    }
    this.#elapsedMilliseconds = Math.max(this.#elapsedMilliseconds, storedMilliseconds);
  }

  #requireMonotonicInstant(value: number): void {
    if (!Number.isFinite(value) || value < 0) {
      throw new RangeError("Monotonic instant must be finite and nonnegative.");
    }
    if (this.#startedAtMilliseconds !== null && value < this.#startedAtMilliseconds) {
      throw new RangeError("Question display clock cannot move backwards.");
    }
  }
}
