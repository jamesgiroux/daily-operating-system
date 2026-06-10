/** @vitest-environment jsdom */

import fs from "node:fs";
import path from "node:path";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ReactBlockRenderer } from "@/components/composition/ReactBlockRenderer";
import { BLOCK_RENDERERS } from "@/components/composition/blocks/BlockComponents";
import {
  KNOWN_COMPOSITION_BLOCK_TYPES,
  type ProjectedBlock,
} from "@/services/composition/contracts";

const { submitMock } = vi.hoisted(() => ({
  submitMock: vi.fn(async () => true),
}));

vi.mock("@/hooks/useIntelligenceCorrection", () => ({
  useIntelligenceCorrection: () => ({
    submitting: false,
    success: false,
    error: null,
    submit: submitMock,
    reset: () => {},
  }),
}));

function block(overrides: Partial<ProjectedBlock> = {}): ProjectedBlock {
  return {
    block_id: "block-fixture",
    block_index: 0,
    original_type_id: "account_overview",
    selected_known_type_id: "account_overview",
    payload: {
      account: { display_name: "Example Account" },
      summary: "A grounded account summary.",
      vitals: [{ label: "Lifecycle", value: "active" }],
      context: [],
    },
    banner: null,
    trust_band: "likely_current",
    claim_refs: [],
    provenance: [],
    edit_routes: [],
    diagnostics: [],
    ...overrides,
  };
}

describe("ReactBlockRenderer", () => {
  beforeEach(() => {
    submitMock.mockClear();
  });

  it("keeps frontend renderer coverage exhaustive with Rust BlockType", () => {
    const rustSource = fs.readFileSync(
      path.resolve(process.cwd(), "src-tauri/abilities-runtime/src/abilities/composition.rs"),
      "utf8",
    );
    const blockTypeImpl = rustSource.match(/impl BlockType \{([\s\S]*?)\n\}/);
    const typeIdMatch = blockTypeImpl?.[1].match(/pub fn type_id\(&self\) -> &str \{[\s\S]*?match self \{([\s\S]*?)\n\s*\}/);
    expect(typeIdMatch?.[1]).toBeTruthy();

    const rustKnownTypeIds = [...(typeIdMatch?.[1] ?? "").matchAll(/Self::[A-Za-z0-9_]+\s*=>\s*"([^"]+)"/g)]
      .map((match) => match[1])
      .sort();

    expect(Object.keys(BLOCK_RENDERERS).sort()).toEqual(rustKnownTypeIds);
  });

  it("maps every known composition block type to a renderer", () => {
    expect(Object.keys(BLOCK_RENDERERS).sort()).toEqual(
      [...KNOWN_COMPOSITION_BLOCK_TYPES].sort(),
    );
  });

  it("renders the chrome-free account overview hero (identity + vitals, no lede or per-line provenance)", () => {
    render(
      <ReactBlockRenderer
        block={block({
          payload: {
            account: { display_name: "Example Account", type: "customer" },
            summary: "A grounded account summary.",
            vitals: [{ label: "Lifecycle", value: "active", display_value: "active" }],
            context: [],
          },
          provenance: [{ invocation_id: "invocation-fixture", field_path: "/summary" }],
        })}
        renderedProvenance={{
          surface: "tauri_app",
          value: {
            produced_at: "2026-06-01T10:00:00Z",
            field_attributions: { "/summary": { source_refs: [{ source: { source_index: 0 } }, { source: { source_index: 1 } }] } },
            about_this: { summary: { source_count: 2 } },
          },
        }}
      />,
    );

    expect(screen.getByText("Example Account")).toBeInTheDocument();
    // Dot-strip composes "<label> <value>" into one cell.
    expect(screen.getByText("Lifecycle active")).toBeInTheDocument();
    // Identity-only hero: no summary lede, and no per-line provenance on the
    // hero — the ambient freshness dot is the only trust signal; sources live
    // in the sources chapter (R3 trust-surfacing decision).
    expect(screen.queryByText("A grounded account summary.")).not.toBeInTheDocument();
    expect(screen.queryByText("from 2 sources")).not.toBeInTheDocument();
  });

  it("renders claim feedback affordances for allowed edit routes", () => {
    render(
      <ReactBlockRenderer
        accountId="acct-feedback"
        block={block({
          selected_known_type_id: "claim_summary",
          payload: {
            title: "Current signal",
            text: "The account risk is rising.",
            trust_band: "use_with_caution",
          },
          claim_refs: [{ claim_id: "claim-feedback", claim_version: 3, field_path: "/text" }],
          edit_routes: [
            {
              field_path: "/text",
              role: "feedback_target",
              claim_refs: [{ claim_id: "claim-feedback", claim_version: 3, field_path: "/text" }],
              feedback_allowed: true,
              refusal_reason: null,
            },
          ],
        })}
      />,
    );

    expect(screen.getByText("Is this accurate?")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Yes" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Partially" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "No" })).toBeInTheDocument();
    expect(screen.queryByText("/text")).not.toBeInTheDocument();
  });

  it("routes claim feedback to the provided composition entity", async () => {
    render(
      <ReactBlockRenderer
        entityId="project-feedback"
        entityType="project"
        block={block({
          selected_known_type_id: "claim_summary",
          payload: {
            title: "Current signal",
            text: "The project risk is rising.",
            trust_band: "use_with_caution",
          },
          claim_refs: [{ claim_id: "claim-feedback", claim_version: 3, field_path: "/text" }],
          edit_routes: [
            {
              field_path: "/text",
              role: "feedback_target",
              claim_refs: [{ claim_id: "claim-feedback", claim_version: 3, field_path: "/text" }],
              feedback_allowed: true,
              refusal_reason: null,
            },
          ],
        })}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Yes" }));

    await waitFor(() =>
      expect(submitMock).toHaveBeenCalledWith({
        entityId: "project-feedback",
        entityType: "project",
        field: "composition:text",
        action: "confirmed",
        source: undefined,
      }),
    );
  });

  it("surfaces account snapshot degradation without exposing the internal reason", () => {
    render(
      <ReactBlockRenderer
        block={block({
          payload: {
            account: { display_name: "Example Account" },
            summary: "A grounded account summary.",
            snapshot_degraded: "account_snapshot_unavailable",
            vitals: [],
            context: [],
          },
        })}
      />,
    );

    expect(screen.getByText("Account details unavailable")).toBeInTheDocument();
    expect(screen.getByText("Some sourced account details could not be loaded for this view.")).toBeInTheDocument();
    expect(screen.queryByText("account_snapshot_unavailable")).not.toBeInTheDocument();
  });

  // Provenance resolution + masking live in BlockShell, which the chrome-free
  // hero no longer uses. Exercise that invariant through a BlockShell-rendering
  // block (claim_summary) so the coverage survives the hero redesign.
  function shellBlock(overrides: Partial<ProjectedBlock> = {}): ProjectedBlock {
    return block({
      selected_known_type_id: "claim_summary",
      payload: { title: "Current signal", text: "Body", trust_band: "likely_current" },
      ...overrides,
    });
  }

  it("keeps resolved provenance quiet (trust is opacity, not a source-count chip)", () => {
    render(
      <ReactBlockRenderer
        block={shellBlock({
          provenance: [{ invocation_id: "invocation-fixture", field_path: "/sections/8/blocks/0" }],
        })}
        renderedProvenance={{
          surface: "tauri_app",
          value: {
            produced_at: "2026-06-01T10:00:00Z",
            field_attributions: {
              "/sections/8/blocks/0/payload/text": { source_refs: [{ source: { source_index: 0 } }] },
            },
          },
        }}
      />,
    );

    // Resolved provenance no longer surfaces a routine "from N sources" chip on
    // the block face (sources live in the sources chapter) — but it must NOT be
    // mis-flagged as pending. The content renders.
    expect(screen.queryByText("from 1 source")).not.toBeInTheDocument();
    expect(screen.queryByText("Source pending")).not.toBeInTheDocument();
    expect(screen.getByText("Body")).toBeInTheDocument();
  });

  it("does not mark valid blocks pending when provenance attributions are truncated", () => {
    render(
      <ReactBlockRenderer
        block={shellBlock({
          provenance: [{ invocation_id: "invocation-fixture", field_path: "/sections/18/blocks/0" }],
        })}
        renderedProvenance={{
          surface: "tauri_app",
          value: {
            warnings: [{ kind: "truncated_for_render" }],
            about_this: { summary: { source_count: 7 } },
          },
        }}
      />,
    );

    // Truncated-but-valid provenance stays quiet and is not mis-flagged pending.
    expect(screen.queryByText("from 7 sources")).not.toBeInTheDocument();
    expect(screen.queryByText("Source pending")).not.toBeInTheDocument();
    expect(screen.getByText("Body")).toBeInTheDocument();
  });

  it("renders missing and masked provenance states without diagnostics", () => {
    render(
      <>
        <ReactBlockRenderer
          block={shellBlock({
            block_id: "missing-provenance",
            provenance: [{ invocation_id: "invocation-fixture", field_path: "/sections/8/blocks/0" }],
          })}
        />
        <ReactBlockRenderer
          block={shellBlock({
            block_id: "masked-provenance",
            provenance: [{ invocation_id: "invocation-fixture", field_path: "/sections/8/blocks/0" }],
          })}
          renderedProvenance={{
            surface: "tauri_app",
            value: { kind: "provenance_masked", status: "masked" },
          }}
        />
      </>,
    );

    expect(screen.getByText("Provenance unavailable")).toBeInTheDocument();
    expect(screen.getByText("Provenance masked")).toBeInTheDocument();
    // Raw internal field pointers must never leak to the surface.
    expect(screen.queryByText("/sections/8/blocks/0")).not.toBeInTheDocument();
  });

  it("renders fallback banner without exposing diagnostics", () => {
    render(
      <ReactBlockRenderer
        block={block({
          original_type_id: "dailyos/private-custom",
          selected_known_type_id: "claim_summary",
          banner: "Rendered as nearest known type — payload may be incomplete.",
          payload: {
            title: "Fallback title",
            body: "Safe fallback body",
            trust_band: "needs_verification",
          },
          diagnostics: [
            {
              diagnostic_kind: "payload_dropped",
              reason: "sensitive_pointer_removed",
              dropped_pointer_count: 2,
              block_id: "block-fixture",
              original_type_id: "dailyos/private-custom",
              selected_known_type_id: "claim_summary",
            },
          ],
        })}
      />,
    );

    expect(screen.getByText("Fallback")).toBeInTheDocument();
    expect(screen.getByText("Fallback title")).toBeInTheDocument();
    expect(screen.queryByText("sensitive_pointer_removed")).not.toBeInTheDocument();
    expect(screen.queryByText("dailyos/private-custom")).not.toBeInTheDocument();
    expect(screen.getByRole("article")).not.toHaveAttribute("data-original-block-type");
    expect(screen.getByRole("article")).not.toHaveAttribute("data-block-id");
  });

  it("renders generic text fallback blocks instead of dropping the banner", () => {
    render(
      <ReactBlockRenderer
        block={block({
          original_type_id: "dailyos/private-custom",
          selected_known_type_id: "dailyos/text",
          banner: "Rendered as dailyos/text — payload may be incomplete",
          payload: {
            text: "Safe generic fallback body",
            trust_band: "needs_verification",
          },
        })}
      />,
    );

    expect(screen.getByText("Fallback")).toBeInTheDocument();
    expect(screen.getByText("Safe generic fallback body")).toBeInTheDocument();
    expect(screen.queryByText("Unavailable block")).not.toBeInTheDocument();
  });

  it("holds the unknown-block fallback privacy boundary in edit mode (ADR-0136 §3.1)", () => {
    render(
      <ReactBlockRenderer
        editMode
        accountId="acct-edit"
        block={block({
          original_type_id: "dailyos/private-custom",
          selected_known_type_id: "claim_summary",
          banner: "Rendered as nearest known type — payload may be incomplete.",
          payload: {
            title: "Fallback title",
            body: "Safe fallback body",
            trust_band: "needs_verification",
          },
          diagnostics: [
            {
              diagnostic_kind: "payload_dropped",
              reason: "sensitive_pointer_removed",
              dropped_pointer_count: 2,
              block_id: "block-fixture",
              original_type_id: "dailyos/private-custom",
              selected_known_type_id: "claim_summary",
            },
          ],
        })}
      />,
    );

    // Edit mode must not relax the §3.1 fallback privacy boundary: the banner
    // shows, but the internal diagnostic reason, the original (private) block
    // type, and identifying data-* attributes never leak — same guarantee as
    // read mode, now asserted with editMode engaged.
    expect(screen.getByText("Fallback")).toBeInTheDocument();
    expect(screen.getByText("Fallback title")).toBeInTheDocument();
    expect(screen.queryByText("sensitive_pointer_removed")).not.toBeInTheDocument();
    expect(screen.queryByText("dailyos/private-custom")).not.toBeInTheDocument();
    expect(screen.getByRole("article")).not.toHaveAttribute("data-original-block-type");
    expect(screen.getByRole("article")).not.toHaveAttribute("data-block-id");
  });
});
