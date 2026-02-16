import { useState, useEffect, useRef } from "react";
import { isScrolling } from "@/lib/smooth-scroll";

export function useChapterObserver(chapterIds: string[], ready = true): [string, (id: string) => void] {
  const [activeId, setActiveId] = useState(chapterIds[0] ?? "");
  const observerRef = useRef<IntersectionObserver | null>(null);

  useEffect(() => {
    setActiveId(chapterIds[0] ?? "");
  }, [chapterIds]);

  useEffect(() => {
    if (chapterIds.length === 0) return;
    if (!ready) return;

    observerRef.current = new IntersectionObserver(
      (entries) => {
        if (isScrolling) return;
        const intersecting = entries.filter((e) => e.isIntersecting);
        if (intersecting.length > 0) {
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
  }, [chapterIds, ready]);

  return [activeId, setActiveId];
}
