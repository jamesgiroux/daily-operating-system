/**
 * Feedback-kind constants for dailyos/suggested-next-steps.
 *
 * These strings are the TypeScript-side authoritative data-feedback-kind
 * values for W3-A. They map onto ADR-0123 RecommendationFeedbackDecision
 * variants while the view script keeps feedback submission behind an explicit
 * disabled guard until W4-A.
 */

export const SUGGESTED_NEXT_STEPS_FEEDBACK_KINDS = {
	convert: {
		decisionKind: "convert",
		adrVariant: "RecommendationFeedbackDecision::Convert",
	},
	dismiss: {
		decisionKind: "dismiss",
		dismissReason: "notRelevant",
		adrVariant: "RecommendationFeedbackDecision::Dismiss",
	},
	dismissWithReason: {
		decisionKind: "dismiss",
		dismissReason: "other",
		adrVariant: "RecommendationFeedbackDecision::Dismiss",
		requiresNote: true,
	},
	notUseful: {
		decisionKind: "notUseful",
		adrVariant: "RecommendationFeedbackDecision::NotUseful",
	},
	tooNoisy: {
		decisionKind: "tooNoisy",
		adrVariant: "RecommendationFeedbackDecision::TooNoisy",
	},
} as const;

export type SuggestedNextStepsFeedbackKind = keyof typeof SUGGESTED_NEXT_STEPS_FEEDBACK_KINDS;

export const SUGGESTED_NEXT_STEPS_FEEDBACK_KIND_VALUES = Object.keys(
	SUGGESTED_NEXT_STEPS_FEEDBACK_KINDS,
) as SuggestedNextStepsFeedbackKind[];
