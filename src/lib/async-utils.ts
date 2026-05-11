/**
 * Race a promise against a timeout. If the timeout fires first, the returned
 * promise rejects with `new Error(timeoutMessage)`.
 *
 * The underlying promise is not cancelled — callers that need cancellation
 * should layer it on top (e.g. AbortController). This helper exists so the
 * caller surfaces a useful message instead of hanging forever.
 */
export function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number,
  timeoutMessage = "Operation timed out",
): Promise<T> {
  return Promise.race([
    promise,
    new Promise<never>((_, reject) => {
      setTimeout(() => reject(new Error(timeoutMessage)), timeoutMs);
    }),
  ]);
}
