/**
 * Local hooks for the Monthly Wrapped page.
 */
import type React from "react";
import { useState, useEffect } from "react";

/**
 * useSlideActive — fires once when a slide enters viewport (30% threshold).
 * Used by each slide component to trigger entrance animations.
 */
export function useSlideActive(id: string) {
  const [active, setActive] = useState(false);
  useEffect(() => {
    const el = document.getElementById(id);
    if (!el) return;
    const obs = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) setActive(true);
      },
      { threshold: 0.3 },
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, [id]);
  return active;
}

/**
 * anim — resolves animation className + optional --anim-delay CSS variable.
 *
 * When `active` is false, returns `.animHidden` (opacity: 0).
 * When `active` is true, returns the appropriate animation class and,
 * if a non-zero delay is provided, a style object setting --anim-delay.
 */
export function anim(
  s: Record<string, string>,
  active: boolean,
  type: "slideUp" | "slideUpSlow" | "slideUpLong" | "fadeIn" | "fadeInSlow" | "scaleReveal",
  delay?: string,
): { className: string; style?: React.CSSProperties } {
  const classMap: Record<string, string> = {
    slideUp: s.animSlideUp,
    slideUpSlow: s.animSlideUpSlow,
    slideUpLong: s.animSlideUpLong,
    fadeIn: s.animFadeIn,
    fadeInSlow: s.animFadeInSlow,
    scaleReveal: s.animScaleReveal,
  };
  if (!active) return { className: s.animHidden };
  const result: { className: string; style?: React.CSSProperties } = { className: classMap[type] };
  if (delay && delay !== "0ms") {
    result.style = { "--anim-delay": delay } as React.CSSProperties;
  }
  return result;
}
