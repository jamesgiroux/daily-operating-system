/**
 * Custom smooth scroll with editorial easing.
 *
 * Uses ease-in-out cubic easing over 800ms — slower and more
 * deliberate than the browser's default `scroll-behavior: smooth`
 * (~300ms), matching the v3 mockup's reading rhythm.
 */

function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

const DURATION_MS = 800;

/** True while a programmatic smooth scroll is animating. */
export let isScrolling = false;

export function smoothScrollTo(elementId: string, offset = 48): void {
  const el = document.getElementById(elementId);
  if (!el) return;

  const targetY =
    el.getBoundingClientRect().top + window.scrollY - offset;
  const startY = window.scrollY;
  const distance = targetY - startY;

  if (Math.abs(distance) < 1) return;

  let startTime: number | null = null;

  isScrolling = true;

  function step(timestamp: number) {
    if (startTime === null) startTime = timestamp;
    const elapsed = timestamp - startTime;
    const progress = Math.min(elapsed / DURATION_MS, 1);
    const easedProgress = easeInOutCubic(progress);

    window.scrollTo(0, startY + distance * easedProgress);

    if (progress >= 1) {
      isScrolling = false;
    } else {
      requestAnimationFrame(step);
    }
  }

  requestAnimationFrame(step);
}
