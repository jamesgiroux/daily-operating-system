/**
 * AnimatedNumber — counts from 0 to `value` over `duration`ms when it enters
 * the viewport. Uses IntersectionObserver so the count-up only fires once the
 * number is actually visible (scroll-snap friendly).
 */
import { useState, useEffect, useRef } from "react";

interface AnimatedNumberProps {
  value: number;
  duration?: number;
  style?: React.CSSProperties;
}

export function AnimatedNumber({ value, duration = 1500, style }: AnimatedNumberProps) {
  const [displayed, setDisplayed] = useState(0);
  const [active, setActive] = useState(false);
  const ref = useRef<HTMLSpanElement>(null);

  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry.isIntersecting) setActive(true);
      },
      { threshold: 0.5 },
    );
    if (ref.current) observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (!active) return;
    let start = 0;
    const step = Math.max(1, Math.ceil(value / (duration / 16)));
    const timer = setInterval(() => {
      start += step;
      if (start >= value) {
        setDisplayed(value);
        clearInterval(timer);
      } else {
        setDisplayed(start);
      }
    }, 16);
    return () => clearInterval(timer);
  }, [active, value, duration]);

  return (
    <span ref={ref} style={style}>
      {displayed.toLocaleString()}
    </span>
  );
}
