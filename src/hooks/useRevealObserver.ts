import { useEffect, useRef } from "react";

/**
 * Observes `.editorial-reveal` elements and adds `.visible`
 * when they scroll into the viewport. Mirrors the v3 mockup's
 * IntersectionObserver fade-in system (600ms ease, 16px translateY).
 *
 * Pass a boolean `ready` flag — the observer sets up when content is rendered.
 * Optional `revision` value forces re-observation when data reloads (e.g. after
 * a workflow refresh) even though `ready` stays true.
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

      const reveals = document.querySelectorAll(".editorial-reveal:not(.visible), .editorial-reveal-slow:not(.visible)");
      reveals.forEach((el) => observerRef.current!.observe(el));
    }, 50);

    return () => {
      clearTimeout(timer);
      observerRef.current?.disconnect();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, revision]);
}
