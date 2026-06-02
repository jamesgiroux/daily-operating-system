import { EditableText } from "@/components/ui/EditableText";
import { useIntelligenceCorrection } from "@/hooks/useIntelligenceCorrection";
import type {
  CompositionFeedbackEntityType,
  EditRoute,
} from "@/services/composition/contracts";

interface CompositionInlineEditProps {
  accountId?: string;
  entityType?: CompositionFeedbackEntityType;
  route: EditRoute | null;
  value: string;
  as?: "span" | "p" | "h1" | "h2" | "h3" | "div";
  multiline?: boolean;
  className?: string;
  fallback: JSX.Element;
}

function pointerSegment(segment: string): string {
  return segment.replace(/~1/g, "/").replace(/~0/g, "~");
}

function feedbackField(route: EditRoute): string {
  const normalized = route.field_path
    .split("/")
    .filter(Boolean)
    .map(pointerSegment)
    .join(".");
  return normalized ? `composition:${normalized}` : "composition:block";
}

export function CompositionInlineEdit({
  accountId,
  entityType = "account",
  route,
  value,
  as = "span",
  multiline = true,
  className,
  fallback,
}: CompositionInlineEditProps) {
  const { submit } = useIntelligenceCorrection();
  const claimRef = route?.claim_refs[0];
  if (!accountId || !route?.feedback_allowed || !claimRef) return fallback;

  return (
    <EditableText
      value={value}
      as={as}
      multiline={multiline}
      className={className}
      onChange={async (correctedValue) => {
        const ok = await submit({
          entityId: accountId,
          entityType,
          field: feedbackField(route),
          action: "corrected",
          itemKey: claimRef.claim_id,
          currentValue: value,
          correctedValue,
          source: "composition_inline_edit",
        });
        if (!ok) throw new Error("Could not save correction");
      }}
    />
  );
}
