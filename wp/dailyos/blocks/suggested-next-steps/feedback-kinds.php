<?php
/**
 * Feedback-kind constants for dailyos/suggested-next-steps.
 *
 * These strings are the PHP-side authoritative data-feedback-kind values for
 * W3-A. They map onto ADR-0123 RecommendationFeedbackDecision variants while
 * keeping the disabled W3-A affordance markup stable until the W4-A submit
 * path enables writes.
 *
 * @package DailyOS
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	return '';
}

if ( ! defined( 'DAILYOS_SUGGESTED_NEXT_STEPS_FEEDBACK_KINDS' ) ) {
	define(
		'DAILYOS_SUGGESTED_NEXT_STEPS_FEEDBACK_KINDS',
		[
			'convert'           => [
				'decisionKind' => 'convert',
				'adrVariant'   => 'RecommendationFeedbackDecision::Convert',
			],
			'dismiss'           => [
				'decisionKind' => 'dismiss',
				'dismissReason' => 'notRelevant',
				'adrVariant'   => 'RecommendationFeedbackDecision::Dismiss',
			],
			'dismissWithReason' => [
				'decisionKind'  => 'dismiss',
				'dismissReason' => 'other',
				'adrVariant'    => 'RecommendationFeedbackDecision::Dismiss',
				'requiresNote'  => true,
			],
			'notUseful'         => [
				'decisionKind' => 'notUseful',
				'adrVariant'   => 'RecommendationFeedbackDecision::NotUseful',
			],
			'tooNoisy'          => [
				'decisionKind' => 'tooNoisy',
				'adrVariant'   => 'RecommendationFeedbackDecision::TooNoisy',
			],
		]
	);
}

if ( ! function_exists( 'dailyos_suggested_next_steps_feedback_kind_map' ) ) {
	/**
	 * Return the ADR-0123 feedback-kind map for this block.
	 *
	 * @return array<string, array<string, mixed>>
	 */
	function dailyos_suggested_next_steps_feedback_kind_map(): array {
		return DAILYOS_SUGGESTED_NEXT_STEPS_FEEDBACK_KINDS;
	}
}

if ( ! function_exists( 'dailyos_suggested_next_steps_feedback_kinds' ) ) {
	/**
	 * Return the allowed data-feedback-kind values.
	 *
	 * @return array<int, string>
	 */
	function dailyos_suggested_next_steps_feedback_kinds(): array {
		return array_keys( dailyos_suggested_next_steps_feedback_kind_map() );
	}
}
