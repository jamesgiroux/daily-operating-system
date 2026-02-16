/**
 * EditorialLoading — pulsing placeholder blocks for editorial pages.
 * Extracted from ActionsPage for reuse across all editorial surfaces.
 */
export function EditorialLoading({ count = 4 }: { count?: number }) {
  return (
    <div style={{ maxWidth: 900, marginLeft: "auto", marginRight: "auto", paddingTop: 80 }}>
      {Array.from({ length: count }).map((_, i) => (
        <div
          key={i}
          style={{
            height: 60,
            background: "var(--color-rule-light)",
            borderRadius: 8,
            marginBottom: 12,
            animation: "pulse 1.5s ease-in-out infinite",
          }}
        />
      ))}
    </div>
  );
}
