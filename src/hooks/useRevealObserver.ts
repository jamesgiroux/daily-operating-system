import { useEffect, useRef } from "react";

/**
 * Observes `.editorial-reveal` elements and adds `.visible`
 * when they scroll into the viewport. Mirrors the v3 mockup's
 * IntersectionObserver fade-in system (600ms ease, 16px translateY).
 *
 * Pass a boolean `ready` flag — the observer sets up when content is rendered.
 *
 * Optional `revision` value forces re-observation when the DOM surface
 * changes shape without the consumer remounting:
 *   - A workflow refresh that swaps new content into the same reveals.
 *   - A tab/view switch where the other view's elements were in the DOM
 *     but `display: none` at initial observe time (IntersectionObserver
 *     can't fire on a non-laid-out subtree, so they'd stay invisible
 *     forever). Pass the active view key as `revision` — changing it
 *     tears down the prior observer and queries the DOM fresh, picking
 *     up the now-displayed reveals.
 *
 * Already-`.visible` elements are excluded from the fresh query, so
 * re-observation doesn't re-trigger fades on content the user has
 * already seen.
 */
export function useRevealObserver(ready: boolean, revision?: unknown) {
  const observerRef = useRef<IntersectionObserver | null>(null);

  useEffect(() => {
    if (!ready) return;

    // Small delay to let React finish painting reveals into the DOM
    const timer = setTimeout(() => {
      observerRef.current = new IntersectionObserver(
        (entries) => {
          entries.forEach((entry) => {
            if (entry.isIntersecting) {
              entry.target.classList.add("visible");
              observerRef.current?.unobserve(entry.target);
            }
          });
        },
        { threshold: 0.08, rootMargin: "0px 0px -40px 0px" }
      );

      const reveals = document.querySelectorAll(".editorial-reveal:not(.visible), .editorial-reveal-slow:not(.visible), .editorial-reveal-stagger:not(.visible)");
      reveals.forEach((el) => observerRef.current!.observe(el));
    }, 50);

    return () => {
      clearTimeout(timer);
      observerRef.current?.disconnect();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, revision]);
}
