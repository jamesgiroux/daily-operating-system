import { describe, expect, it } from "vitest";

import type {
  ClaimReceipt,
  ClaimSensitivity,
  ClaimState,
  ClaimVerificationState,
  FeedbackAction,
  Freshness,
  ReceiptTarget,
  RedactionAffordance,
  RedactionLevel,
  RenderPolicyKind,
  RenderSurface,
  SubjectRef,
  SurfaceContext,
  SurfacingState,
  TrustBand,
} from "../contracts";

interface ContractGoldenFixture {
  receipts: ClaimReceipt[];
  enumCoverage: {
    receiptTargets: ReceiptTarget[];
    surfaceContexts: SurfaceContext[];
    freshness: Freshness[];
    redactionLevels: RedactionLevel[];
    subjectRefs: SubjectRef[];
    trustBands: TrustBand[];
    claimStates: ClaimState[];
    surfacingStates: SurfacingState[];
    verificationStates: ClaimVerificationState[];
    feedbackActions: FeedbackAction[];
    renderPolicyKinds: RenderPolicyKind[];
    claimSensitivities: ClaimSensitivity[];
    renderSurfaces: RenderSurface[];
    redactionAffordances: RedactionAffordance[];
  };
}

const goldenFixture = {
  receipts: [
    {
      target: {
        kind: "claim",
        claimId: "claim-1",
        subject: { account: "account-1" },
        fieldPath: "health.summary",
      },
      surfaceContext: "actions_work",
      renderedText: {
        text: "Account health is stable.",
        policy: {
          kind: "render",
          sensitivity: "internal",
          surface: "action",
          claimId: "claim-1",
          affordance: null,
        },
      },
      trust: {
        band: "likely_current",
        sourceAsof: "2026-05-19T12:00:00Z",
        freshness: "current",
        caveat: null,
        rationale: "Fresh corroborated source.",
      },
      lifecycle: {
        claimState: "active",
        surfacingState: "active",
        verificationState: "active",
        updatedAt: "2026-05-19T12:05:00Z",
      },
      provenance: {
        sources: [
          {
            label: "CRM note",
            sourceType: "crm",
            asOf: "2026-05-19T12:00:00Z",
            href: null,
            redacted: true,
          },
        ],
        fieldPath: "health.summary",
        evidenceSummary: "Latest CRM note supports the claim.",
        redaction: "partial",
      },
      actions: [
        {
          action: "confirm_current",
          label: "Still current",
          disabledReason: null,
        },
      ],
    },
    {
      target: {
        kind: "proposal",
        proposalId: "proposal-1",
        subject: { project: "project-1" },
        fieldPath: null,
      },
      surfaceContext: "entity_detail",
      renderedText: {
        text: "Confidential claim hidden",
        policy: {
          kind: "redacted",
          sensitivity: "confidential",
          surface: "tauri_entity_detail",
          claimId: "claim-2",
          affordance: {
            kind: "confidential_click_to_reveal",
            claim_id: "claim-2",
            label: "Confidential claim hidden",
            audit_required: true,
          },
        },
      },
      trust: {
        band: "use_with_caution",
        sourceAsof: null,
        freshness: "aging",
        caveat: "Source timestamp missing.",
        rationale: null,
      },
      lifecycle: {
        claimState: "dormant",
        surfacingState: "dormant",
        verificationState: "contested",
        updatedAt: null,
      },
      provenance: {
        sources: [],
        fieldPath: null,
        evidenceSummary: null,
        redaction: "full",
      },
      actions: [
        {
          action: "wrong_source",
          label: "Wrong source",
          disabledReason: "Source identity is hidden here.",
        },
      ],
    },
    {
      target: {
        kind: "workItem",
        actionId: "action-1",
        backingClaimId: null,
        subject: { multi: [{ person: "person-1" }, "global"] },
      },
      surfaceContext: "mcp",
      renderedText: null,
      trust: {
        band: "unscored",
        sourceAsof: null,
        freshness: "unknown",
        caveat: null,
        rationale: "No scored evidence yet.",
      },
      lifecycle: {
        claimState: "withdrawn",
        surfacingState: "active",
        verificationState: "needs_user_decision",
        updatedAt: "2026-05-18T09:00:00Z",
      },
      provenance: {
        sources: [
          {
            label: "Meeting transcript",
            sourceType: null,
            asOf: null,
            href: "dailyos://meeting/meeting-1",
            redacted: false,
          },
        ],
        fieldPath: "nextSteps.0",
        evidenceSummary: "Work item was derived from meeting notes.",
        redaction: "none",
      },
      actions: [
        {
          action: "cannot_verify",
          label: "Cannot verify",
          disabledReason: null,
        },
      ],
    },
  ],
  enumCoverage: {
    receiptTargets: [
      {
        kind: "claim",
        claimId: "claim-target",
        subject: { account: "account-target" },
        fieldPath: "field",
      },
      {
        kind: "proposal",
        proposalId: "proposal-target",
        subject: { meeting: "meeting-target" },
        fieldPath: null,
      },
      {
        kind: "workItem",
        actionId: "action-target",
        backingClaimId: "claim-target",
        subject: "unknown",
      },
    ],
    surfaceContexts: [
      "actions_work",
      "entity_detail",
      "daily_briefing",
      "meeting_detail",
      "mcp",
    ],
    freshness: ["current", "aging", "stale", "unknown"],
    redactionLevels: ["none", "partial", "full"],
    subjectRefs: [
      { account: "account-1" },
      { project: "project-1" },
      { person: "person-1" },
      { meeting: "meeting-1" },
      { user: "user-1" },
      "global",
      { multi: [{ account: "account-1" }, { meeting: "meeting-1" }] },
      "unknown",
    ],
    trustBands: [
      "likely_current",
      "use_with_caution",
      "needs_verification",
      "unscored",
    ],
    claimStates: ["active", "dormant", "tombstoned", "withdrawn"],
    surfacingStates: ["active", "dormant"],
    verificationStates: ["active", "contested", "needs_user_decision"],
    feedbackActions: [
      "confirm_current",
      "mark_outdated",
      "mark_false",
      "wrong_subject",
      "wrong_source",
      "cannot_verify",
      "needs_nuance",
      "surface_inappropriate",
      "not_relevant_here",
    ],
    renderPolicyKinds: ["render", "redacted", "drop"],
    claimSensitivities: ["public", "internal", "confidential", "user_only"],
    renderSurfaces: [
      "tauri_entity_detail",
      "tauri_briefing_prep",
      "tauri_meeting_detail",
      "tauri_email_summary",
      "action",
      "tauri_provenance",
      "tauri_report",
      "tauri_chat",
      "mcp_tool",
      "mcp_tool_detail",
      "p2_publication",
      "log_structured",
      "push_notification",
    ],
    redactionAffordances: [
      {
        kind: "confidential_click_to_reveal",
        claim_id: "claim-2",
        label: "Confidential claim hidden",
        audit_required: true,
      },
      {
        kind: "confidential_hidden",
        label: "Confidential claim hidden",
      },
      {
        kind: "user_only_hidden",
        label: "User-only claim hidden",
      },
    ],
  },
} satisfies ContractGoldenFixture;

describe("claim receipt contract golden fixture", () => {
  it("parses representative receipt JSON as ClaimReceipt", () => {
    const parsedFixture = JSON.parse(JSON.stringify(goldenFixture)) as ContractGoldenFixture;
    const receipts = parsedFixture.receipts.map((receipt) => receipt);

    expect(receipts.map((receipt) => receipt.target.kind)).toEqual([
      "claim",
      "proposal",
      "workItem",
    ]);
    expect(receipts[0]?.surfaceContext).toBe("actions_work");
    expect(receipts[0]?.actions[0]?.action).toBe("confirm_current");
    expect(receipts[0]?.provenance.sources[0]?.redacted).toBe(true);
  });

  it("covers every closed enum variant mirrored from Rust", () => {
    const parsedFixture = JSON.parse(JSON.stringify(goldenFixture)) as ContractGoldenFixture;
    const { enumCoverage } = parsedFixture;

    expect(enumCoverage.receiptTargets.map((target) => target.kind)).toEqual([
      "claim",
      "proposal",
      "workItem",
    ]);
    expect(enumCoverage.surfaceContexts).toHaveLength(5);
    expect(enumCoverage.freshness).toHaveLength(4);
    expect(enumCoverage.redactionLevels).toHaveLength(3);
    expect(enumCoverage.subjectRefs).toHaveLength(8);
    expect(enumCoverage.trustBands).toHaveLength(4);
    expect(enumCoverage.claimStates).toHaveLength(4);
    expect(enumCoverage.surfacingStates).toHaveLength(2);
    expect(enumCoverage.verificationStates).toHaveLength(3);
    expect(enumCoverage.feedbackActions).toHaveLength(9);
    expect(enumCoverage.renderPolicyKinds).toHaveLength(3);
    expect(enumCoverage.claimSensitivities).toHaveLength(4);
    expect(enumCoverage.renderSurfaces).toHaveLength(13);
    expect(enumCoverage.redactionAffordances.map((affordance) => affordance.kind)).toEqual([
      "confidential_click_to_reveal",
      "confidential_hidden",
      "user_only_hidden",
    ]);
  });
});
