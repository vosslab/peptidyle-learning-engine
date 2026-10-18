export const cleanupGraceMs = 2_000;

export function createCaptureLifecycle({ signal, timeoutMs, onFatal }) {
  const controller = new AbortController();
  const abort = () => controller.abort(new Error("capture_aborted"));
  const timer = setTimeout(() => controller.abort(new Error("capture_deadline")), timeoutMs);
  signal.addEventListener("abort", abort, { once: true });
  if (signal.aborted) abort();

  let cleanupPromise;
  const fatal =
    onFatal ??
    ((error) => {
      process.stderr.write(`thumbnail helper cleanup failed: ${error.message}\n`);
      process.exit(1);
    });

  function requireActive() {
    if (controller.signal.aborted) throw controller.signal.reason ?? new Error("capture_aborted");
  }

  async function boundedCleanup(cleanup) {
    const operation = Promise.resolve().then(cleanup);
    let timerId;
    const expired = new Promise((_, reject) => {
      timerId = setTimeout(() => reject(new Error("capture_cleanup_timeout")), cleanupGraceMs);
    });
    try {
      await Promise.race([operation, expired]);
    } catch (error) {
      fatal(error);
      await operation.catch(() => {});
      throw error;
    } finally {
      clearTimeout(timerId);
    }
  }

  async function phase(promise, lateCleanup = async () => {}) {
    const operation = Promise.resolve(promise);
    operation.catch(() => {});
    let abortHandler;
    const aborted = new Promise((_, reject) => {
      abortHandler = () => reject(controller.signal.reason ?? new Error("capture_aborted"));
      controller.signal.addEventListener("abort", abortHandler, { once: true });
      if (controller.signal.aborted) abortHandler();
    });
    try {
      const result = await Promise.race([operation, aborted]);
      requireActive();
      return result;
    } catch (error) {
      if (controller.signal.aborted) {
        cleanupPromise ??= boundedCleanup(async () => {
          await operation.then(
            (result) => lateCleanup(result),
            () => {},
          );
        });
        await cleanupPromise;
      }
      throw error;
    } finally {
      controller.signal.removeEventListener("abort", abortHandler);
    }
  }

  async function cleanup(action) {
    if (!action) return;
    await boundedCleanup(action);
  }

  function close() {
    clearTimeout(timer);
    signal.removeEventListener("abort", abort);
  }

  return { signal: controller.signal, phase, cleanup, close, requireActive };
}
