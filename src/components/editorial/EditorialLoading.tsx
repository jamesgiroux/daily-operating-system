/**
 * EditorialLoading — skeleton placeholder for editorial pages.
 *
 * Follows the margin grid pattern (100px label column + content) so the
 * loading state maps spatially to the actual content it precedes. No rounded
 * corners — ADR-0077 uses rectangular shapes throughout.
 */

function Pulse({
  w,
  h,
  mb,
  mt,
}: {
  w?: number | string;
  h: number;
  mb?: number;
  mt?: number;
}) {
  return (
    <div
      style={{
        width: w ?? "100%",
        height: h,
        marginBottom: mb,
        marginTop: mt,
        background: "var(--color-rule-light)",
        borderRadius: 2,
        animation: "pulse 1.5s ease-in-out infinite",
      }}
    />
  );
}

export function EditorialLoading({ count = 4 }: { count?: number }) {
  return (
    <div style={{ paddingTop: 72 }}>
      {Array.from({ length: count }).map((_, i) => (
        <div
          key={i}
          style={{
            display: "grid",
            gridTemplateColumns: "100px 32px 1fr",
            marginBottom: 40,
          }}
        >
          {/* Label column pulse */}
          <div style={{ paddingTop: 4 }}>
            <Pulse w={52} h={10} />
          </div>

          <div />

          {/* Content column */}
          <div>
            {/* Section rule */}
            <div
              style={{
                borderTop: "1px solid var(--color-rule-light)",
                marginBottom: 20,
              }}
            />
            {/* Title line */}
            <Pulse w={i === 0 ? "70%" : `${50 + i * 8}%`} h={i === 0 ? 22 : 16} mb={10} />
            {/* Body lines */}
            <Pulse h={14} mb={6} />
            <Pulse w="80%" h={14} />
          </div>
        </div>
      ))}
    </div>
  );
}
