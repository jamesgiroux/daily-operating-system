<?php
/**
 * DailyOS Mock Runtime Client — dev-only intercept for visual development.
 *
 * Provides canned `invoke_ability` responses for the DailyOS blocks so
 * Studio can render the WP surfaces WITHOUT a paired Tauri runtime + real
 * SQLCipher data. Copy this file to `wp-content/mu-plugins/dailyos-block-
 * showcase.php` to enable; the plugin's own runtime client still wins when
 * paired (mock falls through for unmocked ability names).
 *
 * **NOT shipped in the plugin.** This file is the canonical source so
 * Studio environments can stay in sync — symlink or copy at activation
 * time. Tracked under `wp/dailyos/dev-tools/` so it survives across
 * Studio instances and rebuilds.
 *
 * Intercepts the `dailyos_runtime_client_for_block` filter to install a
 * composite client that:
 *
 *   1. For ability names listed in `mock_response()`, returns a canned
 *      envelope that matches the real producer's serialized shape (per
 *      `src-tauri/abilities-runtime/src/abilities/<ability>/contracts.rs`).
 *
 *   2. For any other ability, falls through to the real plugin client when
 *      paired, or returns `WP_Error('mock_unhandled', ...)` when not.
 *
 * Canned data deliberately mirrors the personas in the design system
 * reference HTML (Acme Corp renewal checkpoint, Priya Raman, etc) so block
 * renders match the reference visuals when wired to the design tokens.
 *
 * @package DailyOSDev
 */

declare(strict_types=1);

if ( ! defined( 'ABSPATH' ) ) {
	exit;
}

// Priority 100 — installed AFTER the plugin's default_runtime_client_for_block
// (priority 5). The composite always wins because it wraps whatever the
// plugin set, so mock + real coexist.
add_filter(
	'dailyos_runtime_client_for_block',
	static function ( $existing ) {
		return new DailyOS_Mock_Runtime_Client( $existing );
	},
	100
);

// Provide a default scope set when nothing is paired so the inner blocks'
// `dailyos_surfaceclient_resolved_scopes` filter returns a non-empty array.
// The mock client ignores scopes; this exists so any code paths that gate
// on "do we have ANY scopes" don't short-circuit.
add_filter(
	'dailyos_surfaceclient_resolved_scopes',
	static function ( $scopes ) {
		if ( is_array( $scopes ) && ! empty( $scopes ) ) {
			return $scopes;
		}
		return [
			'read.entity_intelligence',
			'read.account_intelligence',
			'read.project_intelligence',
			'read.person_intelligence',
			'read.meeting_intelligence',
			'read.briefing',
			'read.open_loops',
			'read.touchpoints',
		];
	},
	100
);

/**
 * Composite runtime client wrapping the real plugin client (or null when
 * unpaired). All ability invocations are routed through `invoke_ability`;
 * unrelated methods fall through to the inner client via `__call`.
 */
final class DailyOS_Mock_Runtime_Client {
	private mixed $inner;

	public function __construct( mixed $inner ) {
		$this->inner = $inner;
	}

	/**
	 * Ability dispatch. Returns canned response for mocked abilities;
	 * delegates to inner client for unmocked ones; returns WP_Error when
	 * no inner client is paired.
	 */
	public function invoke_ability( string $name, array $payload, array $scope_set ): array|\WP_Error {
		$mock = $this->mock_response( $name, $payload );
		if ( null !== $mock ) {
			return $mock;
		}
		if ( is_object( $this->inner ) && method_exists( $this->inner, 'invoke_ability' ) ) {
			return $this->inner->invoke_ability( $name, $payload, $scope_set );
		}
		return new \WP_Error(
			'mock_unhandled',
			sprintf( 'Mock has no canned response for ability `%s` and no inner client is paired.', $name )
		);
	}

	/**
	 * Forward any other method to the inner client (project_composition,
	 * pairing helpers, etc).
	 */
	public function __call( string $name, array $args ) {
		if ( is_object( $this->inner ) && method_exists( $this->inner, $name ) ) {
			return $this->inner->$name( ...$args );
		}
		return new \WP_Error(
			'mock_no_inner',
			sprintf( 'Method `%s` not mocked and no inner client is paired.', $name )
		);
	}

	private function mock_response( string $name, array $payload ): ?array {
		switch ( $name ) {
			case 'get_entity_intelligence':
				return self::wrap( 'get_entity_intelligence', DailyOS_Mock_Data::entity_intelligence( $payload ) );
			case 'meeting_prep_status':
				return self::wrap( 'meeting_prep_status', DailyOS_Mock_Data::meeting_prep_status( $payload ) );
			case 'get_daily_briefing':
				return self::wrap( 'get_daily_briefing', DailyOS_Mock_Data::daily_briefing( $payload ) );
			case 'claim_receipt':
				return self::wrap( 'claim_receipt', DailyOS_Mock_Data::claim_receipt( $payload ) );
			case 'list_accounts':
				return self::wrap( 'list_accounts', DailyOS_Mock_Data::list_accounts( $payload ) );
			case 'list_open_loops':
				return self::wrap( 'list_open_loops', DailyOS_Mock_Data::list_open_loops( $payload ) );
		}
		return null;
	}

	/**
	 * Wrap a canned data payload in the runtime's response envelope shape
	 * per `src-tauri/src/bridges/types.rs::AbilityResponseJson`.
	 */
	private static function wrap( string $ability_name, ?array $data ): array {
		if ( null === $data ) {
			return [
				'ok'         => false,
				'request_id' => 'mock-' . uniqid( '', true ),
				'error'      => [
					'code'    => 'mock_no_canned_data',
					'message' => sprintf( 'Mock has no canned data shape for `%s`.', $ability_name ),
				],
			];
		}
		return [
			'ok'         => true,
			'request_id' => 'mock-' . uniqid( '', true ),
			'ability'    => [
				'ability_name' => $ability_name,
				'data'         => $data,
			],
		];
	}
}

/**
 * Canned envelopes for each mocked ability. Shapes mirror the producer
 * contracts at `src-tauri/abilities-runtime/src/abilities/<name>/contracts.rs`.
 * Personas mirror the design reference (Acme Corp / Priya Raman / etc).
 */
final class DailyOS_Mock_Data {

	// ---- get_entity_intelligence ----------------------------------------

	public static function entity_intelligence( array $payload ): array {
		$entity_type = isset( $payload['entity_type'] ) ? (string) $payload['entity_type'] : '';
		$entity_id   = isset( $payload['entity_id'] ) ? (string) $payload['entity_id'] : '';

		return match ( $entity_type ) {
			'meeting' => self::meeting_envelope( $entity_id ),
			'account' => self::account_envelope( $entity_id ),
			'project' => self::project_envelope( $entity_id ),
			'person'  => self::person_envelope( $entity_id ),
			default   => self::empty_envelope( $entity_type, $entity_id ),
		};
	}

	private static function meeting_envelope( string $entity_id ): array {
		$id = '' === $entity_id ? 'mtg-acme-renewal-checkpoint' : $entity_id;
		return [
			'schemaVersion' => 1,
			'envelopeRenderId' => 'mock-env-' . $id,
			'subject' => [
				'kind' => 'meeting',
				'id'   => $id,
				'name' => 'Acme Corp renewal checkpoint',
			],
			'sections' => self::sections_all_present(),
			'facts' => [
				'items' => [
					[ 'key' => 'title', 'value' => 'Acme Corp renewal checkpoint', 'trust_band' => 'likely_current' ],
					[ 'key' => 'time_local', 'value' => '10:00 AM - 10:45 AM', 'trust_band' => 'likely_current' ],
					[ 'key' => 'duration_minutes', 'value' => '45', 'trust_band' => 'likely_current' ],
					[ 'key' => 'meeting_type', 'value' => 'customer', 'trust_band' => 'likely_current' ],
					[ 'key' => 'primary_account', 'value' => 'Acme Corp', 'trust_band' => 'likely_current' ],
					[ 'key' => 'organizer', 'value' => 'Priya Raman', 'trust_band' => 'likely_current' ],
					[ 'key' => 'attendee_count', 'value' => '6', 'trust_band' => 'likely_current' ],
				],
				'next_cursor' => null,
				'total_hint'  => 7,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			// Attendees + risks + plan items are surfaced through the
			// `record_entries` paginated list. The meeting-attendees-section,
			// meeting-recommended-actions, meeting-context-bundle blocks
			// project specific record_kind filters from this list.
			'attendees' => self::meeting_attendees(),
			'risks' => self::meeting_risks(),
			'recent_wins' => self::meeting_recent_wins(),
			'readiness_items' => self::meeting_readiness_items(),
			'recommended_actions' => self::meeting_recommended_actions(),
			'post_meeting_intelligence' => self::meeting_post_intel(),
			'health_story' => [
				'summary' => 'Acme validated the Q2 Launch path and narrowed the renewal risk to one legal owner and one launch dependency.',
				'trust_band' => 'likely_current',
				'aggregate_score' => 71,
			],
			'metadata_proposals' => self::empty_paginated(),
			'open_loops' => [
				'items' => [
					self::open_loop_item( 'send-msa-redlines', 'Send Acme Corp final MSA redlines to Jen Park', 'overdue', '~15m' ),
					self::open_loop_item( 'confirm-sponsor-coverage', 'Confirm sponsor coverage before legal review', 'today', '~10m' ),
				],
				'next_cursor' => null,
				'total_hint' => 2,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'touchpoints' => [
				'items' => [
					self::touchpoint_item( 'tp-acme-prep-apr21', 'Acme Corp commercial prep', '2026-04-21T15:00:00Z', 'past', 4 ),
					self::touchpoint_item( 'tp-acme-renewal-now', 'Acme Corp renewal checkpoint', '2026-05-22T10:00:00Z', 'present', 6 ),
				],
				'next_cursor' => null,
				'total_hint' => 2,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'threads' => [
				'items' => [
					[ 'thread_id' => 'th-msa-redlines', 'headline' => 'Review final MSA redlines', 'detail' => 'due Apr 23', 'kind' => 'open' ],
					[ 'thread_id' => 'th-workflow-rehearsal', 'headline' => 'Finalize support workflow rehearsal', 'detail' => 'closed since the last meeting', 'kind' => 'confirmed' ],
					[ 'thread_id' => 'th-health-delta', 'headline' => 'Health moved from 74 to 82', 'detail' => '', 'kind' => 'neutral' ],
					[ 'thread_id' => 'th-new-attendee-sara', 'headline' => 'Sara Wu', 'detail' => 'new attendee', 'kind' => 'new_face' ],
				],
				'next_cursor' => null,
				'total_hint' => 4,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'record_entries' => self::empty_paginated(),
			'trust' => [
				'aggregate_band' => 'likely_current',
				'likely_current_count' => 7,
				'use_with_caution_count' => 0,
				'needs_verification_count' => 0,
			],
			'provenance' => self::provenance_now(),
			'sensitivity' => [ 'kind' => 'normal' ],
		];
	}

	private static function account_envelope( string $entity_id ): array {
		$id = '' === $entity_id ? 'acme-corp' : $entity_id;
		return [
			'schemaVersion' => 1,
			'envelopeRenderId' => 'mock-env-' . $id,
			'subject' => [
				'kind' => 'account',
				'id'   => $id,
				'name' => 'Acme Corp',
			],
			'sections' => self::sections_all_present(),
			'facts' => [
				'items' => [
					[ 'key' => 'lifecycle', 'value' => 'renewing', 'trust_band' => 'likely_current' ],
					[ 'key' => 'health', 'value' => 'yellow', 'trust_band' => 'likely_current' ],
					[ 'key' => 'arr', 'value' => '$280,000', 'trust_band' => 'likely_current' ],
					[ 'key' => 'renewal_date', 'value' => '2026-07-15', 'trust_band' => 'likely_current' ],
					[ 'key' => 'stage', 'value' => 'negotiating', 'trust_band' => 'likely_current' ],
					[ 'key' => 'nps', 'value' => '32', 'trust_band' => 'use_with_caution' ],
				],
				'next_cursor' => null,
				'total_hint' => 6,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'health_story' => [
				'summary' => 'Procurement joined the renewal thread and asked for security evidence before approving the extension.',
				'trust_band' => 'use_with_caution',
				'aggregate_score' => 71,
			],
			'metadata_proposals' => self::empty_paginated(),
			'open_loops' => [
				'items' => [
					self::open_loop_item( 'sec-questionnaire-owner', 'Security questionnaire needs an owner before renewal review', 'overdue', '~25m' ),
					self::open_loop_item( 'send-pricing-appendix', 'Send revised pricing appendix', 'overdue', '~10m' ),
					self::open_loop_item( 'confirm-attendees', 'Confirm attendee list for renewal call', 'done', '' ),
				],
				'next_cursor' => null,
				'total_hint' => 3,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'touchpoints' => [
				'items' => [
					self::touchpoint_item( 'tp-acme-renewal-now', 'Acme Corp renewal checkpoint', '2026-05-22T10:00:00Z', 'present', 6 ),
					self::touchpoint_item( 'tp-acme-prep-apr21', 'Acme Corp commercial prep', '2026-04-21T15:00:00Z', 'past', 4 ),
					self::touchpoint_item( 'tp-acme-eom-review', 'Acme Corp end-of-month review', '2026-03-28T14:00:00Z', 'past', 5 ),
				],
				'next_cursor' => null,
				'total_hint' => 3,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'threads' => self::empty_paginated(),
			'record_entries' => self::empty_paginated(),
			'trust' => [
				'aggregate_band' => 'use_with_caution',
				'likely_current_count' => 5,
				'use_with_caution_count' => 1,
				'needs_verification_count' => 0,
			],
			'provenance' => self::provenance_now(),
			'sensitivity' => [ 'kind' => 'normal' ],
		];
	}

	private static function project_envelope( string $entity_id ): array {
		$id = '' === $entity_id ? 'beta-migration' : $entity_id;
		return [
			'schemaVersion' => 1,
			'envelopeRenderId' => 'mock-env-' . $id,
			'subject' => [
				'kind' => 'project',
				'id'   => $id,
				'name' => 'Beta Migration',
			],
			'sections' => self::sections_all_present(),
			'facts' => [
				'items' => [
					[ 'key' => 'status', 'value' => 'active', 'trust_band' => 'likely_current' ],
					[ 'key' => 'trajectory', 'value' => 'improving', 'trust_band' => 'likely_current' ],
					[ 'key' => 'parent_account', 'value' => 'Acme Corp', 'trust_band' => 'likely_current' ],
					[ 'key' => 'milestone', 'value' => 'Q2 Launch', 'trust_band' => 'likely_current' ],
					[ 'key' => 'target_date', 'value' => '2026-06-30', 'trust_band' => 'likely_current' ],
				],
				'next_cursor' => null,
				'total_hint' => 5,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'health_story' => [
				'summary' => 'Beta migration thread is warming up again after two quiet weeks; champion forwarded the migration plan.',
				'trust_band' => 'likely_current',
				'aggregate_score' => 78,
			],
			'metadata_proposals' => self::empty_paginated(),
			'open_loops' => [
				'items' => [
					self::open_loop_item( 'audit-log-export-owner', 'Confirm audit-log export owner', 'today', '~15m' ),
				],
				'next_cursor' => null,
				'total_hint' => 1,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'touchpoints' => self::empty_paginated(),
			'threads' => self::empty_paginated(),
			'record_entries' => self::empty_paginated(),
			'trust' => [
				'aggregate_band' => 'likely_current',
				'likely_current_count' => 5,
				'use_with_caution_count' => 0,
				'needs_verification_count' => 0,
			],
			'provenance' => self::provenance_now(),
			'sensitivity' => [ 'kind' => 'normal' ],
		];
	}

	private static function person_envelope( string $entity_id ): array {
		$id = '' === $entity_id ? 'priya-raman' : $entity_id;
		return [
			'schemaVersion' => 1,
			'envelopeRenderId' => 'mock-env-' . $id,
			'subject' => [
				'kind' => 'person',
				'id'   => $id,
				'name' => 'Priya Raman',
			],
			'sections' => self::sections_all_present(),
			'facts' => [
				'items' => [
					[ 'key' => 'display_name', 'value' => 'Priya Raman', 'trust_band' => 'likely_current' ],
					[ 'key' => 'role', 'value' => 'Director of Engineering', 'trust_band' => 'likely_current' ],
					[ 'key' => 'organization', 'value' => 'Acme Corp', 'trust_band' => 'likely_current' ],
					[ 'key' => 'email', 'value' => 'priya@subsidiary.com', 'trust_band' => 'likely_current' ],
					[ 'key' => 'last_seen', 'value' => '2026-05-22T10:00:00Z', 'trust_band' => 'likely_current' ],
				],
				'next_cursor' => null,
				'total_hint' => 5,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'health_story' => null,
			'metadata_proposals' => self::empty_paginated(),
			'open_loops' => self::empty_paginated(),
			'touchpoints' => self::empty_paginated(),
			'threads' => self::empty_paginated(),
			'record_entries' => self::empty_paginated(),
			'trust' => [
				'aggregate_band' => 'likely_current',
				'likely_current_count' => 5,
				'use_with_caution_count' => 0,
				'needs_verification_count' => 0,
			],
			'provenance' => self::provenance_now(),
			'sensitivity' => [ 'kind' => 'normal' ],
		];
	}

	private static function empty_envelope( string $entity_type, string $entity_id ): array {
		return [
			'schemaVersion' => 1,
			'envelopeRenderId' => 'mock-env-empty-' . $entity_type . '-' . $entity_id,
			'subject' => [
				'kind' => $entity_type,
				'id'   => $entity_id,
				'name' => sprintf( 'Unknown %s', $entity_type ),
			],
			'sections' => self::sections_all_empty( 'unknown_entity' ),
			'facts' => self::empty_paginated(),
			'health_story' => null,
			'metadata_proposals' => self::empty_paginated(),
			'open_loops' => self::empty_paginated(),
			'touchpoints' => self::empty_paginated(),
			'threads' => self::empty_paginated(),
			'record_entries' => self::empty_paginated(),
			'trust' => [
				'aggregate_band' => 'unscored',
				'likely_current_count' => 0,
				'use_with_caution_count' => 0,
				'needs_verification_count' => 0,
			],
			'provenance' => self::provenance_now(),
			'sensitivity' => [ 'kind' => 'normal' ],
		];
	}

	// ---- meeting_prep_status --------------------------------------------

	public static function meeting_prep_status( array $payload ): array {
		$meeting_id = isset( $payload['meeting_id'] ) ? (string) $payload['meeting_id'] : 'mtg-acme-renewal-checkpoint';
		return [
			'meeting_id' => $meeting_id,
			'event_id' => 'evt-' . $meeting_id,
			'linked_entity_type' => 'account',
			'linked_entity_id' => 'acme-corp',
			'status' => 'ready',
			'blocking_reason' => null,
			'stale_reason' => null,
			'last_prepared_at' => '2026-05-22T09:42:00Z',
			'source_asof_inputs' => [
				[ 'source' => 'gmail', 'as_of' => '2026-05-22T09:42:00Z' ],
				[ 'source' => 'calendar', 'as_of' => '2026-05-22T09:30:00Z' ],
			],
		];
	}

	// ---- get_daily_briefing ---------------------------------------------

	public static function daily_briefing( array $payload ): array {
		return [
			'schemaVersion' => 1,
			'date' => '2026-05-22',
			'state' => [
				'availability' => [ 'kind' => 'available' ],
				'freshness' => [ 'kind' => 'fresh' ],
				'integrity' => [ 'kind' => 'clean' ],
				'advisories' => [],
			],
			'current_meeting' => null,
			'next_meeting' => self::meeting_brief_ref( 'mtg-acme-renewal-checkpoint', 'Acme renewal call', '10:00 AM', '45m', 'customer' ),
			'upcoming_meetings' => [
				'items' => [
					self::meeting_brief_ref( 'mtg-product-strategy', 'Product strategy sync', '1:30 PM', '25m', 'internal' ),
					self::meeting_brief_ref( 'mtg-globex-vendor-review', 'Globex vendor review', '3:00 PM', '30m', 'cancelled' ),
				],
				'next_cursor' => null,
				'total_hint' => 2,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'candidate_set' => [ 'meeting_ids' => [] ],
			'watch_proposals' => [],
			'trust_summary' => [
				'aggregate_band' => 'likely_current',
				'likely_current_count' => 12,
				'use_with_caution_count' => 2,
				'needs_verification_count' => 0,
			],
			'provenance' => self::provenance_now(),
			'sensitivity' => [ 'kind' => 'normal' ],
			'source_asof_inputs' => [
				[ 'source' => 'gmail', 'as_of' => '2026-05-22T09:42:00Z' ],
				[ 'source' => 'calendar', 'as_of' => '2026-05-22T09:30:00Z' ],
			],
		];
	}

	// ---- claim_receipt --------------------------------------------------

	public static function claim_receipt( array $payload ): array {
		return [
			'rows' => [],
			'audience' => 'meeting',
		];
	}

	// ---- list_accounts / list_open_loops --------------------------------

	public static function list_accounts( array $payload ): array {
		return [
			'items' => [
				[ 'account_id' => 'acme-corp', 'name' => 'Acme Corp', 'status' => 'renewing', 'health_band' => 'use_with_caution', 'last_touchpoint_at' => '2026-05-22T10:00:00Z', 'open_loops_count' => 3 ],
				[ 'account_id' => 'globex-holdings', 'name' => 'Globex Holdings', 'status' => 'paused', 'health_band' => 'needs_verification', 'last_touchpoint_at' => '2026-05-18T14:00:00Z', 'open_loops_count' => 1 ],
				[ 'account_id' => 'northstar-inc', 'name' => 'Northstar Inc', 'status' => 'active', 'health_band' => 'likely_current', 'last_touchpoint_at' => '2026-05-21T16:30:00Z', 'open_loops_count' => 0 ],
			],
			'total_after_filter' => 3,
			'data_shifted_advisory' => null,
		];
	}

	public static function list_open_loops( array $payload ): array {
		return [
			'claims' => [
				[ 'claim_id' => 'cl-msa-redlines', 'subject_kind' => 'account', 'subject_id' => 'acme-corp', 'headline' => 'Send Acme Corp final MSA redlines', 'urgency' => 'overdue' ],
				[ 'claim_id' => 'cl-sponsor-coverage', 'subject_kind' => 'account', 'subject_id' => 'acme-corp', 'headline' => 'Confirm sponsor coverage', 'urgency' => 'today' ],
			],
		];
	}

	// ---- helpers --------------------------------------------------------

	private static function sections_all_present(): array {
		$sections = [];
		foreach ( [ 'facts', 'health', 'metadata_proposals', 'open_loops', 'touchpoints', 'threads', 'record' ] as $section ) {
			$sections[ $section ] = [ 'kind' => 'present', 'item_count' => 1 ];
		}
		return $sections;
	}

	private static function sections_all_empty( string $reason ): array {
		$sections = [];
		foreach ( [ 'facts', 'health', 'metadata_proposals', 'open_loops', 'touchpoints', 'threads', 'record' ] as $section ) {
			$sections[ $section ] = [ 'kind' => 'empty', 'reason' => $reason ];
		}
		return $sections;
	}

	private static function empty_paginated(): array {
		return [
			'items' => [],
			'next_cursor' => null,
			'total_hint' => 0,
			'cursor_state' => [ 'kind' => 'stable' ],
		];
	}

	private static function open_loop_item( string $id, string $headline, string $urgency, string $estimate ): array {
		return [
			'claim_id' => 'cl-' . $id,
			'headline' => $headline,
			'urgency' => $urgency,
			'estimate' => $estimate,
			'subject_kind' => 'account',
			'subject_id' => 'acme-corp',
			'receipt' => [ 'rows' => [], 'audience' => 'open_loops' ],
		];
	}

	private static function touchpoint_item( string $id, string $title, string $when, string $side, int $attendee_count ): array {
		return [
			'touchpoint_id' => 'tp-' . $id,
			'meeting_id' => 'mtg-' . $id,
			'title' => $title,
			'when' => $when,
			'side' => $side,
			'attendee_count' => $attendee_count,
			'inclusion_reason' => 'subject_match',
		];
	}

	private static function meeting_brief_ref( string $meeting_id, string $title, string $time_local, string $duration, string $kind ): array {
		return [
			'meeting_id' => $meeting_id,
			'title' => $title,
			'time_local' => $time_local,
			'duration' => $duration,
			'meeting_kind' => $kind,
			'attendee_count' => 6,
			'primary_account' => 'Acme Corp',
		];
	}

	// ---- meeting-surface specific data ---------------------------------

	private static function meeting_attendees(): array {
		return [
			[
				'person_id' => 'jen-park',
				'display_name' => 'Jen Park',
				'avatar_initial' => 'J',
				'avatar_style' => 'default',
				'role' => 'VP Customer Operations',
				'organization' => 'Acme Corp',
				'temperature' => 'warm',
				'engagement' => 'champion',
				'assessment' => 'Jen will sponsor the executive readout if the redline owner is explicit and the Q2 Launch value story stays concrete.',
				'meeting_count' => 6,
				'last_seen_label' => 'Last seen yesterday',
				'tooltip_assessment' => 'Executive sponsor; wants the commercial story before legal review.',
			],
			[
				'person_id' => 'dan-mitchell',
				'display_name' => 'Dan Mitchell',
				'avatar_initial' => 'D',
				'avatar_style' => 'cold',
				'role' => 'Procurement Lead',
				'organization' => 'Acme Corp',
				'temperature' => 'cold',
				'engagement' => 'detractor',
				'assessment' => 'Dan needs a bounded owner and timeline before he will stop reopening MSA language.',
				'meeting_count' => 3,
				'last_seen_label' => 'Last seen 2 weeks ago',
			],
			[
				'person_id' => 'sara-wu',
				'display_name' => 'Sara Wu',
				'avatar_initial' => 'S',
				'avatar_style' => 'new',
				'role' => 'Technical Approver',
				'organization' => 'Acme Corp',
				'temperature' => 'cool',
				'engagement' => 'new_contact',
				'assessment' => 'Sara wants technical risk summarized separately from the commercial story.',
				'meeting_count' => 1,
				'last_seen_label' => 'New contact',
			],
			[
				'person_id' => 'priya-raman',
				'display_name' => 'Priya Raman',
				'avatar_initial' => 'P',
				'avatar_style' => 'default',
				'role' => 'Director of Engineering',
				'organization' => 'Acme Corp',
				'temperature' => 'warm',
				'engagement' => 'supporter',
				'assessment' => 'Priya is the day-to-day partner; will reinforce technical commitments if framed concretely.',
				'meeting_count' => 12,
				'last_seen_label' => 'Last seen 3 days ago',
			],
			[
				'person_id' => 'owen-carter',
				'display_name' => 'Owen Carter',
				'avatar_initial' => 'O',
				'avatar_style' => 'default',
				'role' => 'Legal Counsel',
				'organization' => 'Acme Corp',
				'temperature' => 'cool',
				'engagement' => 'tentative',
				'assessment' => 'Tentative attendee; only joins for procurement discussion.',
				'meeting_count' => 2,
				'last_seen_label' => 'Last seen 6 days ago',
			],
			[
				'person_id' => 'james-giroux',
				'display_name' => 'James Giroux',
				'avatar_initial' => 'J',
				'avatar_style' => 'self',
				'role' => 'Customer Success Lead',
				'organization' => 'DailyOS',
				'temperature' => 'self',
				'engagement' => 'self',
				'assessment' => '',
				'meeting_count' => 24,
				'last_seen_label' => 'You',
			],
		];
	}

	private static function meeting_risks(): array {
		return [
			[
				'rank' => 'featured',
				'urgency' => 'high',
				'text' => 'If the MSA redlines leave this meeting without one named owner, procurement will treat the renewal as slipping and reopen pricing pressure.',
				'claim_id' => 'cl-risk-msa-redlines',
			],
			[
				'rank' => 'subordinate',
				'urgency' => 'high',
				'text' => 'Dan has not accepted audit-log ownership; if you leave it implied, the Q2 Launch plan will still have a visible gap.',
				'claim_id' => 'cl-risk-audit-log-owner',
			],
			[
				'rank' => 'subordinate',
				'urgency' => 'medium',
				'text' => 'Sara is new to the renewal path; overloading her with commercial detail before the technical summary could dilute the architecture signal.',
				'claim_id' => 'cl-risk-sara-onboarding',
			],
		];
	}

	private static function meeting_recent_wins(): array {
		return [
			[
				'text' => 'Helpline Rollout rehearsal landed cleanly with support; Acme Corp no longer sees launch readiness as the blocker.',
				'claim_id' => 'cl-win-helpline-rollout',
			],
			[
				'text' => 'Jen volunteered to sponsor the executive readout if the legal owner and launch owner are explicit by end of day.',
				'claim_id' => 'cl-win-jen-sponsor',
			],
		];
	}

	private static function meeting_readiness_items(): array {
		return [
			[
				'text' => 'Have the MSA redline summary ready for Jen, with the two clauses legal still owns.',
				'dot_tone' => 'turmeric_muted',
			],
			[
				'text' => 'Bring the Q2 Launch dependency map so Dan can confirm the audit-log owner live.',
				'dot_tone' => 'turmeric_muted',
			],
			[
				'text' => 'Ask Sara whether the technical risk summary should go to the exec readout or stay in the renewal thread.',
				'dot_tone' => 'turmeric_muted',
			],
		];
	}

	private static function meeting_recommended_actions(): array {
		return [
			[
				'action_id' => 'act-confirm-msa-owner',
				'headline' => 'Confirm the MSA redline owner before procurement re-engages.',
				'urgency' => 'overdue',
				'context' => '~15m · Acme Corp',
				'why' => 'Blocks the legal close-out path Jen named.',
			],
			[
				'action_id' => 'act-publish-launch-deps',
				'headline' => 'Publish Q2 Launch dependency map with named owners.',
				'urgency' => 'today',
				'context' => '~20m · Beta Migration',
				'why' => 'Lets Dan accept audit-log ownership live.',
			],
		];
	}

	private static function meeting_post_intel(): array {
		// Post-meeting intelligence — replaces flat outcomes when available.
		// Referenced by reference HTML lines 59-160 (PostMeetingIntelligence_*).
		return [
			'summary' => 'Acme validated the Q2 Launch path and narrowed the renewal risk to one legal owner and one launch dependency. Jen Park agreed to sponsor the executive follow-up once Dan Mitchell confirms the audit-log export owner.',
			'thread_items' => [
				[ 'kind' => 'confirmed', 'headline' => 'Finalize support workflow rehearsal', 'detail' => 'closed since the last meeting' ],
				[ 'kind' => 'open', 'headline' => 'Review final MSA redlines', 'detail' => 'due Apr 23' ],
				[ 'kind' => 'neutral', 'headline' => 'Health moved from 74 to 82', 'detail' => '' ],
				[ 'kind' => 'new_face', 'headline' => 'Sara Wu', 'detail' => 'new attendee' ],
			],
			'predictions' => [
				'risks' => [
					[ 'matched' => true, 'prediction' => 'MSA redlines could stall procurement', 'reality' => 'Jen asked for one legal owner before procurement sees the package.' ],
					[ 'matched' => false, 'prediction' => 'Pricing pressure would resurface', 'reality' => '' ],
				],
				'opportunities' => [
					[ 'matched' => true, 'prediction' => 'Sara would surface technical depth on the audit-log path', 'reality' => 'Sara confirmed the export contract is the binding architectural decision.' ],
				],
			],
		];
	}

	private static function provenance_now(): array {
		return [
			'source_asof' => '2026-05-22T09:42:00Z',
			'sources' => [
				[ 'source' => 'gmail', 'as_of' => '2026-05-22T09:42:00Z' ],
				[ 'source' => 'calendar', 'as_of' => '2026-05-22T09:30:00Z' ],
			],
		];
	}
}
