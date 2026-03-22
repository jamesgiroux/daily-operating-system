import type { ComponentType } from "react";
import { X } from "lucide-react";
import styles from "./ConnectorDetail.module.css";

interface ConnectorDetailProps {
  component: ComponentType;
  onClose: () => void;
}

export default function ConnectorDetail({ component: Component, onClose }: ConnectorDetailProps) {
  return (
    <div className={styles.container}>
      <div className={styles.panel}>
        <button
          onClick={onClose}
          className={styles.closeButton}
          aria-label="Close detail panel"
        >
          <X size={14} />
        </button>
        <Component />
      </div>
    </div>
  );
}
