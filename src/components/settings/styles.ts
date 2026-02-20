/**
 * Shared editorial styles for Settings components.
 * Extracted from the original SettingsPage monolith (I349).
 */

export const styles = {
  subsectionLabel: {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    fontWeight: 600,
    letterSpacing: "0.06em",
    textTransform: "uppercase" as const,
    color: "var(--color-text-tertiary)",
    margin: 0,
    marginBottom: 12,
  },
  fieldLabel: {
    fontFamily: "var(--font-sans)",
    fontSize: 13,
    fontWeight: 500,
    color: "var(--color-text-secondary)",
    marginBottom: 4,
    display: "block" as const,
  },
  input: {
    width: "100%",
    fontFamily: "var(--font-sans)",
    fontSize: 14,
    color: "var(--color-text-primary)",
    background: "none",
    border: "none",
    borderBottom: "1px solid var(--color-rule-light)",
    padding: "8px 0",
    outline: "none",
  },
  btn: {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    fontWeight: 600,
    letterSpacing: "0.06em",
    textTransform: "uppercase" as const,
    background: "none",
    borderRadius: 4,
    padding: "4px 14px",
    cursor: "pointer",
    transition: "all 0.15s ease",
  },
  btnPrimary: {
    color: "var(--color-garden-olive)",
    border: "1px solid var(--color-garden-olive)",
  },
  btnGhost: {
    color: "var(--color-text-tertiary)",
    border: "1px solid var(--color-rule-heavy)",
  },
  btnDanger: {
    color: "var(--color-spice-terracotta)",
    border: "1px solid var(--color-spice-terracotta)",
  },
  description: {
    fontFamily: "var(--font-sans)",
    fontSize: 13,
    color: "var(--color-text-tertiary)",
    lineHeight: 1.5,
    margin: 0,
  },
  settingRow: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "12px 0",
    borderBottom: "1px solid var(--color-rule-light)",
  },
  statusDot: (color: string) => ({
    width: 8,
    height: 8,
    borderRadius: "50%",
    background: color,
    flexShrink: 0 as const,
  }),
  monoLabel: {
    fontFamily: "var(--font-mono)",
    fontSize: 11,
    fontWeight: 500,
    letterSpacing: "0.04em",
    color: "var(--color-text-tertiary)",
  },
  sectionGap: {
    marginBottom: 48,
  },
  thinRule: {
    height: 1,
    background: "var(--color-rule-light)",
    border: "none",
    margin: "16px 0",
  },
};
