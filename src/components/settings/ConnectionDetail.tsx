import type { ComponentType } from "react";
import { X } from "lucide-react";
import { styles } from "./styles";

interface ConnectionDetailProps {
  component: ComponentType;
  onClose: () => void;
}

export default function ConnectionDetail({ component: Component, onClose }: ConnectionDetailProps) {
  return (
    <div
      style={{
        overflow: "hidden",
        animation: "accordion-open 0.2s ease-out",
      }}
    >
      <div
        style={{
          padding: "24px 0",
          borderBottom: "1px solid var(--color-rule-light)",
          position: "relative",
        }}
      >
        <button
          onClick={onClose}
          style={{
            ...styles.btn,
            position: "absolute",
            top: 16,
            right: 0,
            color: "var(--color-text-tertiary)",
            border: "none",
            padding: "4px",
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
          }}
          aria-label="Close detail panel"
        >
          <X size={14} />
        </button>
        <Component />
      </div>
      <style>{`
        @keyframes accordion-open {
          from {
            max-height: 0;
            opacity: 0;
          }
          to {
            max-height: 1200px;
            opacity: 1;
          }
        }
      `}</style>
    </div>
  );
}
