import { useEffect, useRef } from "react";

const REVEAL_SELECTOR =
  ".editorial-reveal:not(.visible), .editorial-reveal-slow:not(.visible), .editorial-reveal-stagger:not(.visible)";

/**
 * Observes `.editorial-reveal` elements and adds `.visible`
 * when they scroll into the viewport. Mirrors the v3 mockup's
 * IntersectionObserver fade-in system (600ms ease, 16px translateY).
 *
 * Pass a boolean `ready` flag — the observer sets up when content is rendered.
 *
 * Late-mounted reveal nodes are observed through a MutationObserver, so
 * independent async sections don't need to wire themselves into a revision key.
 *
 * Optional `revision` value still forces re-observation when the DOM surface
 * changes layout without new nodes being inserted:
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
  const mutationObserverRef = useRef<MutationObserver | null>(null);
  const observedRef = useRef<WeakSet<Element>>(new WeakSet());

  useEffect(() => {
    if (!ready) return;
    observedRef.current = new WeakSet();

    // Small delay to let React finish painting reveals into the DOM
    const timer = setTimeout(() => {
      const observer = new IntersectionObserver(
        (entries) => {
          entries.forEach((entry) => {
            if (entry.isIntersecting) {
              entry.target.classList.add("visible");
              observer.unobserve(entry.target);
              observedRef.current.delete(entry.target);
            }
          });
        },
        { threshold: 0.08, rootMargin: "0px 0px -40px 0px" }
      );
      observerRef.current = observer;

      const observeReveal = (el: Element) => {
        if (observedRef.current.has(el)) return;
        observer.observe(el);
        observedRef.current.add(el);
      };

      const observeRevealsIn = (root: Element | Document) => {
        if (root instanceof Element && root.matches(REVEAL_SELECTOR)) {
          observeReveal(root);
        }
        root.querySelectorAll(REVEAL_SELECTOR).forEach(observeReveal);
      };

      observeRevealsIn(document);

      if (typeof MutationObserver !== "undefined" && document.body) {
        const mutationObserver = new MutationObserver((mutations) => {
          mutations.forEach((mutation) => {
            mutation.addedNodes.forEach((node) => {
              if (node instanceof Element) {
                observeRevealsIn(node);
              }
            });
          });
        });
        mutationObserver.observe(document.body, { childList: true, subtree: true });
        mutationObserverRef.current = mutationObserver;
      }
    }, 50);

    return () => {
      clearTimeout(timer);
      mutationObserverRef.current?.disconnect();
      mutationObserverRef.current = null;
      observerRef.current?.disconnect();
      observerRef.current = null;
      observedRef.current = new WeakSet();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ready, revision]);
}
