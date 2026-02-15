import { useState, useEffect, useRef } from "react";

/**
 * Tracks which chapter section is currently in view using IntersectionObserver.
 * Returns the ID of the active chapter for nav highlighting.
 */
export function useChapterObserver(chapterIds: string[]): string {
  const [activeId, setActiveId] = useState(chapterIds[0] ?? "");
  const observerRef = useRef<IntersectionObserver | null>(null);

  useEffect(() => {
    if (chapterIds.length === 0) return;

    observerRef.current = new IntersectionObserver(
      (entries) => {
        // Find the entry that is intersecting and closest to the top
        const intersecting = entries.filter((e) => e.isIntersecting);
        if (intersecting.length > 0) {
          // Pick the one with the smallest top boundary (closest to viewport top)
          const top = intersecting.reduce((best, entry) =>
            entry.boundingClientRect.top < best.boundingClientRect.top ? entry : best
          );
          setActiveId(top.target.id);
        }
      },
      { rootMargin: "-40% 0px -60% 0px" }
    );

    const elements = chapterIds
      .map((id) => document.getElementById(id))
      .filter(Boolean) as HTMLElement[];

    elements.forEach((el) => observerRef.current!.observe(el));

    return () => {
      observerRef.current?.disconnect();
    };
  }, [chapterIds]);

  return activeId;
}
