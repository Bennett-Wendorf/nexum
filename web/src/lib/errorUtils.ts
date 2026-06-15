const ERROR_CLEAR_TIMEOUT_MS = 5000;

/**
 * Sets an error message with auto-clear after 5 seconds.
 * The component must declare its own reactive error state:
 *   let error = $state<string | null>(null);
 *
 * Returns a cleanup function that cancels the auto-clear timeout.
 * Call it on component cleanup to prevent memory leaks.
 *
 * @example
 * ```svelte
 * <script lang="ts">
 *   import { onCleanup } from 'svelte';
 *   import { setError } from '$lib/errorUtils';
 *
 *   let error = $state<string | null>(null);
 *
 *   function handleError(message: string): void {
 *     const cleanup = setError(
 *       () => { error = message; },
 *       () => { error = null; }
 *     );
 *     onCleanup(cleanup);
 *   }
 * </script>
 * ```
 */
export function setError(
  onSet: () => void,
  onClear: () => void,
  delayMs: number = ERROR_CLEAR_TIMEOUT_MS
): () => void {
  const timeout = setTimeout(() => {
    onClear();
  }, delayMs);
  onSet();
  return () => clearTimeout(timeout);
}
