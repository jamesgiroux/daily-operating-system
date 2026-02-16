/**
 * EditorialEmpty — serif italic empty state for editorial pages.
 * Extracted from ActionsPage for reuse across all editorial surfaces.
 */
export function EditorialEmpty({ title, message }: { title: string; message?: string }) {
  return (
    <div style={{ textAlign: "center", padding: "64px 0" }}>
      <p
        style={{
          fontFamily: "var(--font-serif)",
          fontSize: 18,
          fontStyle: "italic",
          color: "var(--color-text-tertiary)",
          margin: 0,
        }}
      >
        {title}
      </p>
      {message && (
        <p
          style={{
            fontFamily: "var(--font-sans)",
            fontSize: 13,
            fontWeight: 300,
            color: "var(--color-text-tertiary)",
            marginTop: 8,
          }}
        >
          {message}
        </p>
      )}
    </div>
  );
}
