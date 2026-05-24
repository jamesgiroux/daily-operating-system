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

// Studio/dev seeding: make the parity surfaces reachable as ordinary WP
// posts/pages when this mock client is active as an MU plugin.
add_action( 'init', [ DailyOS_Mock_Data::class, 'seed_mock_surfaces' ], 200 );

/**
 * Composite runtime client wrapping the real plugin client (or null when
 * unpaired). All ability invocations are routed through `invoke_ability`;
 * unrelated methods fall through to the inner client via `__call`.
 */
final class DailyOS_Mock_Runtime_Client {
	/**
	 * Inner runtime client wrapped by the mock.
	 *
	 * @var mixed
	 */
	private mixed $inner;

	/**
	 * Construct the mock runtime client.
	 *
	 * @param mixed $inner Inner runtime client.
	 */
	public function __construct( mixed $inner ) {
		$this->inner = $inner;
	}

	/**
	 * Ability dispatch. Returns canned response for mocked abilities;
	 * delegates to inner client for unmocked ones; returns WP_Error when
	 * no inner client is paired.
	 *
	 * @param string              $name      Ability name.
	 * @param array<string,mixed> $payload   Ability payload.
	 * @param array<int,string>   $scope_set Scope set.
	 * @return array|\WP_Error
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
	 * Legacy v1.4.2 ProjectedComposition mock — intercepts
	 * `project_composition_for_surface` calls with `composition_id`
	 * starting with `showcase-` and returns canned per-block payloads
	 * matching the pre-v1.4.4 `selected_known_type_id` + `payload` shape.
	 *
	 * Used by the restored `dailyos/account-overview` block (resurrected
	 * for compare-against-known-good debugging during v1.4.4 W1 L4).
	 *
	 * @param string      $composition_id      Composition ID.
	 * @param int         $composition_version Composition version.
	 * @param string|null $cache_hint_token    Cache hint token.
	 * @return array<string,mixed>
	 */
	public function project_composition_for_surface(
		string $composition_id,
		int $composition_version = 0,
		?string $cache_hint_token = null
	) {
		if ( 0 === strpos( $composition_id, 'showcase-' ) ) {
			$projection = DailyOS_Mock_Data::canned_projection( substr( $composition_id, strlen( 'showcase-' ) ) );
			if ( null !== $projection ) {
				return [
					'ok'                => true,
					'projection'        => $projection,
					'cache_hint_token'  => 'mock-cache-hint',
					'served_from_cache' => false,
				];
			}
		}
		if ( is_object( $this->inner ) && method_exists( $this->inner, 'project_composition_for_surface' ) ) {
			return $this->inner->project_composition_for_surface( $composition_id, $composition_version, $cache_hint_token );
		}
		return [
			'ok'    => false,
			'error' => [
				'code'    => 'mock_no_canned_composition',
				'message' => sprintf( 'composition_id=%s', $composition_id ),
			],
		];
	}

	/**
	 * Forward any other method (pairing helpers, etc) to the inner client.
	 *
	 * @param string           $name Method name.
	 * @param array<int,mixed> $args Method arguments.
	 * @return mixed|\WP_Error
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

	/**
	 * Get a mock response.
	 *
	 * @param string              $name    Ability name.
	 * @param array<string,mixed> $payload Ability payload.
	 * @return array<string,mixed>|null
	 */
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
	 *
	 * @param string                   $ability_name Ability name.
	 * @param array<string,mixed>|null $data         Ability data.
	 * @return array<string,mixed>
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

// Dev-only mock data class colocated with the mock client (single dev-tools file).
// phpcs:disable Generic.Files.OneObjectStructurePerFile.MultipleFound

/**
 * Canned envelopes for each mocked ability. Shapes mirror the producer
 * contracts at `src-tauri/abilities-runtime/src/abilities/<name>/contracts.rs`.
 * Personas mirror the design reference.
 */
final class DailyOS_Mock_Data {

	/**
	 * Seed the mock CPT posts and ordinary pages used by Studio parity passes.
	 *
	 * This runs only when the dev-only mock client is installed as an MU plugin.
	 * It is intentionally idempotent and limited to known mock slugs.
	 */
	public static function seed_mock_surfaces(): void {
		if (
			! function_exists( 'get_page_by_path' )
			|| ! function_exists( 'wp_insert_post' )
			|| ! function_exists( 'wp_update_post' )
			|| ! function_exists( 'update_post_meta' )
		) {
			return;
		}

		self::seed_entity_post( 'dailyos_account', 'acme-corp', 'Acme Corp', 'acme-corp' );
		self::seed_entity_post( 'dailyos_project', 'beta-migration', 'Beta Migration', 'beta-migration' );
		self::seed_entity_post( 'dailyos_person', 'priya-raman', 'Priya Raman', 'priya-raman' );
		self::seed_entity_post( 'dailyos_meeting', 'mtg-acme-renewal-checkpoint', 'Acme renewal checkpoint', 'mtg-acme-renewal-checkpoint' );
		self::seed_entity_post( 'dailyos_briefing', 'briefing-today', 'Briefing - Today', 'briefing-today' );
		self::seed_page( 'actions', 'Actions' );
		self::seed_page( 'emails', 'The Correspondent' );
		self::seed_page( 'mock-surfaces', 'Mock surfaces' );
		self::maybe_flush_mock_rewrites();
	}

	/**
	 * Seed or update one mock entity CPT post.
	 *
	 * @param string $post_type Post type.
	 * @param string $slug      Post slug.
	 * @param string $title     Post title.
	 * @param string $entity_id Entity ID.
	 */
	private static function seed_entity_post( string $post_type, string $slug, string $title, string $entity_id ): void {
		$post_id = self::seed_post_like( $post_type, $slug, $title );
		if ( 0 < $post_id ) {
			update_post_meta( $post_id, 'dailyos_entity_id', $entity_id );
		}
	}

	/**
	 * Seed or update one mock ordinary page.
	 *
	 * @param string $slug  Page slug.
	 * @param string $title Page title.
	 */
	private static function seed_page( string $slug, string $title ): void {
		self::seed_post_like( 'page', $slug, $title );
	}

	/**
	 * Seed or update a published post-like object by slug and type.
	 *
	 * @param string $post_type Post type.
	 * @param string $slug      Post slug.
	 * @param string $title     Post title.
	 * @return int
	 */
	private static function seed_post_like( string $post_type, string $slug, string $title ): int {
		$existing = self::find_seed_post( $post_type, $slug );
		$postarr  = [
			'post_type'    => $post_type,
			'post_status'  => 'publish',
			'post_name'    => $slug,
			'post_title'   => $title,
			'post_content' => '',
		];

		if ( is_object( $existing ) && isset( $existing->ID ) ) {
			$postarr['ID'] = (int) $existing->ID;
			$result        = wp_update_post( $postarr, true );
			if ( function_exists( 'is_wp_error' ) && is_wp_error( $result ) ) {
				return 0;
			}
			return (int) $existing->ID;
		}

		$result = wp_insert_post( $postarr, true );
		if ( function_exists( 'is_wp_error' ) && is_wp_error( $result ) ) {
			return 0;
		}
		return (int) $result;
	}

	/**
	 * Find a seeded post by exact post_name and type.
	 *
	 * @param string $post_type Post type.
	 * @param string $slug      Post slug.
	 * @return object|null
	 */
	private static function find_seed_post( string $post_type, string $slug ): ?object {
		if ( function_exists( 'get_posts' ) ) {
			// `suppress_filters` is omitted (prohibited by WordPress Plugin Check
			// at literal `true`). get_posts() defaults to suppressing post-query
			// filters; behavior is unchanged.
			$matches = get_posts(
				[
					'name'        => $slug,
					'post_type'   => $post_type,
					'post_status' => 'any',
					'numberposts' => 1,
					'orderby'     => 'ID',
					'order'       => 'ASC',
				]
			);
			if ( is_array( $matches ) && ! empty( $matches ) && is_object( $matches[0] ) ) {
				return $matches[0];
			}
		}

		$existing = get_page_by_path( $slug, OBJECT, $post_type );
		return is_object( $existing ) ? $existing : null;
	}

	/**
	 * Flush rewrite rules once so seeded CPT URLs work in Studio.
	 */
	private static function maybe_flush_mock_rewrites(): void {
		if (
			! function_exists( 'flush_rewrite_rules' )
			|| ! function_exists( 'get_option' )
			|| ! function_exists( 'update_option' )
			|| '1' === (string) get_option( 'dailyos_mock_surfaces_rewrite_flushed', '' )
		) {
			return;
		}

		flush_rewrite_rules( false );
		update_option( 'dailyos_mock_surfaces_rewrite_flushed', '1', false );
	}

	/**
	 * Get entity intelligence.
	 *
	 * @param array<string,mixed> $payload Ability payload.
	 * @return array<string,mixed>
	 */
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

	/**
	 * Build a meeting envelope.
	 *
	 * @param string $entity_id Entity ID.
	 * @return array<string,mixed>
	 */
	private static function meeting_envelope( string $entity_id ): array {
		$id = '' === $entity_id ? 'mtg-acme-renewal-checkpoint' : $entity_id;
		return [
			'schemaVersion'             => 1,
			'envelopeRenderId'          => 'mock-env-' . $id,
			'subject'                   => [
				'kind' => 'meeting',
				'id'   => $id,
				'name' => 'Acme Corp renewal checkpoint',
			],
			'sections'                  => self::sections_all_present(),
			'facts'                     => [
				'items'        => [
					[
						'key'        => 'title',
						'value'      => 'Acme Corp renewal checkpoint',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'time_local',
						'value'      => '10:00 AM - 10:45 AM',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'duration_minutes',
						'value'      => '45',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'meeting_type',
						'value'      => 'customer',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'primary_account',
						'value'      => 'Acme Corp',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'organizer',
						'value'      => 'Priya Raman',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'attendee_count',
						'value'      => '6',
						'trust_band' => 'likely_current',
					],
				],
				'next_cursor'  => null,
				'total_hint'   => 7,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			// Attendees + risks + plan items are surfaced through the
			// `record_entries` paginated list. The meeting-attendees-section,
			// meeting-recommended-actions, meeting-context-bundle blocks
			// project specific record_kind filters from this list.
			'attendees'                 => self::meeting_attendees(),
			'risks'                     => self::meeting_risks(),
			'recent_wins'               => self::meeting_recent_wins(),
			'readiness_items'           => self::meeting_readiness_items(),
			'recommended_actions'       => self::meeting_recommended_actions(),
			'post_meeting_intelligence' => self::meeting_post_intel(),
			'health_story'              => [
				'summary'         => 'Acme validated the Q2 Launch path and narrowed the renewal risk to one legal owner and one launch dependency.',
				'trust_band'      => 'likely_current',
				'aggregate_score' => 71,
			],
			'metadata_proposals'        => self::empty_paginated(),
			'open_loops'                => [
				'items'        => [
					self::open_loop_item( 'send-msa-redlines', 'Send Acme Corp final MSA redlines to Jen Park', 'overdue', '~15m' ),
					self::open_loop_item( 'confirm-sponsor-coverage', 'Confirm sponsor coverage before legal review', 'today', '~10m' ),
				],
				'next_cursor'  => null,
				'total_hint'   => 2,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'touchpoints'               => [
				'items'        => [
					self::touchpoint_item( 'tp-acme-prep-apr21', 'Acme Corp commercial prep', '2026-04-21T15:00:00Z', 'past', 4 ),
					self::touchpoint_item( 'tp-acme-renewal-now', 'Acme Corp renewal checkpoint', '2026-05-22T10:00:00Z', 'present', 6 ),
				],
				'next_cursor'  => null,
				'total_hint'   => 2,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'threads'                   => [
				'items'        => [
					[
						'thread_id' => 'th-msa-redlines',
						'headline'  => 'Review final MSA redlines',
						'detail'    => 'due Apr 23',
						'kind'      => 'open',
					],
					[
						'thread_id' => 'th-workflow-rehearsal',
						'headline'  => 'Finalize support workflow rehearsal',
						'detail'    => 'closed since the last meeting',
						'kind'      => 'confirmed',
					],
					[
						'thread_id' => 'th-health-delta',
						'headline'  => 'Health moved from 74 to 82',
						'detail'    => '',
						'kind'      => 'neutral',
					],
					[
						'thread_id' => 'th-new-attendee-sara',
						'headline'  => 'Sara Wu',
						'detail'    => 'new attendee',
						'kind'      => 'new_face',
					],
				],
				'next_cursor'  => null,
				'total_hint'   => 4,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'record_entries'            => self::empty_paginated(),
			'trust'                     => [
				'aggregate_band'           => 'likely_current',
				'likely_current_count'     => 7,
				'use_with_caution_count'   => 0,
				'needs_verification_count' => 0,
			],
			'provenance'                => self::provenance_now(),
			'sensitivity'               => [ 'kind' => 'normal' ],
		];
	}

	/**
	 * Build an account envelope.
	 *
	 * @param string $entity_id Entity ID.
	 * @return array<string,mixed>
	 */
	private static function account_envelope( string $entity_id ): array {
		$id = '' === $entity_id ? 'acme-corp' : $entity_id;
		// Content mirrors .docs/design/reference/surfaces/account.html (Acme Corp persona).
		// Each fact item carries `renderedText` so envelope_consume_claim() returns
		// the claim directly without fanning out to claim_receipt. trustBand values
		// match ProvenanceTag references in the reference HTML.
		$subject_ref = [
			'kind' => 'account',
			'id'   => $id,
		];
		$fact        = function ( string $cid, string $field, string $text, string $band = 'likely_current', ?string $source = null ) use ( $subject_ref ): array {
			$item = [
				'claimId'      => $cid,
				'fieldPath'    => $field,
				'renderedText' => $text,
				'trustBand'    => $band,
				'sourceAsof'   => '2026-05-22T10:00:00Z',
				'subjectRef'   => $subject_ref,
				'sensitivity'  => [ 'kind' => 'normal' ],
			];
			if ( null !== $source ) {
				$item['dataSource'] = $source;
			}
			return $item;
		};
		return [
			'schemaVersion'        => 1,
			'envelopeRenderId'     => 'mock-env-' . $id,
			'subject'              => [
				'kind' => 'account',
				'id'   => $id,
				'name' => 'Acme Corp',
			],
			'sections'             => self::sections_all_present(),
			'facts'                => [
				'items'        => [
					// EditableVitalsStrip — top of AccountHero (reference lines 107-156).
					$fact( 'cl-acme-arr', 'arr', '$620K ARR', 'likely_current', 'Salesforce' ),
					$fact( 'cl-acme-health', 'health', 'Green Health', 'likely_current', 'Glean CRM' ),
					$fact( 'cl-acme-lifecycle', 'lifecycle', 'Steady-state', 'likely_current' ),
					$fact( 'cl-acme-renewal', 'renewalDate', 'Renewal in 143d', 'likely_current' ),
					$fact( 'cl-acme-nps', 'nps', 'NPS 54', 'use_with_caution', 'Glean CRM' ),
					$fact( 'cl-acme-activity', 'activity', '6 meetings / 30d', 'likely_current' ),

					// Outlook chapter (reference lines 171-238).
					$fact(
						'cl-acme-outlook',
						'agreementOutlook',
						'Renewal confidence is high. Watch for legal review timing and executive sponsor bandwidth. Start the conversation by June 3, 2026.',
						'likely_current',
						'Glean'
					),
					$fact(
						'cl-acme-contract',
						'contractContext',
						'Annual SaaS, no auto-renew. Renews September 24, 2026. Current ARR $620K.',
						'likely_current'
					),
					$fact(
						'cl-acme-expansion-1',
						'expansionSignals[0]',
						'Support workflow expansion is credible now that the admin pilot is active. +$140K ARR potential. Evaluating.',
						'use_with_caution',
						'Glean'
					),

					// Strategic Landscape chapter (reference lines 645-695).
					$fact(
						'cl-acme-company-context',
						'companyContext',
						'Acme Corp is a mid-market services firm consolidating onto shared workflows. Q2 priorities anchor on the renewal cycle and admin pilot proof points.',
						'likely_current'
					),
					$fact(
						'cl-acme-priority-1',
						'strategicPriorities[0]',
						'Move support operations onto the shared workflow. The admin pilot is the proof point for expanding into front-line support. (Sara Wu · Q2, Active)',
						'likely_current'
					),
					$fact(
						'cl-acme-priority-2',
						'strategicPriorities[1]',
						'Keep the renewal commercial story stable before finance review. (Dan Mitchell · June, Evaluating)',
						'use_with_caution'
					),

					// Value & Commitments chapter (reference lines 592-635).
					$fact(
						'cl-acme-value-1',
						'valueDelivered[0]',
						'Support workflow review time dropped after the admin pilot moved into weekly use. (Speed · May 1 · Glean)',
						'likely_current',
						'Glean'
					),
					$fact(
						'cl-acme-value-2',
						'valueDelivered[1]',
						'The renewal thread now has a narrowed legal dependency instead of broad commercial ambiguity. (Risk · Apr 29 · Meeting)',
						'likely_current'
					),
					$fact(
						'cl-acme-metric-1',
						'successMetrics[0]',
						'Admin weekly active use trending up since the pilot opened on April 14.',
						'likely_current'
					),
					$fact(
						'cl-acme-commitment-1',
						'openCommitments[0]',
						'Deliver renewal owner map and security evidence package before the June executive readout.',
						'likely_current'
					),

					// SentimentHero chapter (reference Your Assessment block).
					$fact(
						'cl-acme-executive-assessment',
						'executiveAssessment',
						'Procurement joined the renewal thread and asked for security evidence before approving the extension. Direction: on track with one open legal question.',
						'use_with_caution'
					),
					$fact(
						'cl-acme-pull-quote',
						'pullQuote',
						'"We are engaged again and the renewal feels orderly. Keep the next step focused on legal ownership." — Jen Park, CTO',
						'likely_current'
					),

					// AccountTechnicalFootprint chapter — Products & Entitlements (reference lines 251-298).
					$fact(
						'cl-acme-product-1',
						'products[0]',
						'DailyOS Enterprise — full seat coverage across CS team.',
						'likely_current'
					),
					$fact(
						'cl-acme-product-2',
						'products[1]',
						'Admin Workflow Pilot — opened April 14, 2026.',
						'likely_current'
					),

					// LinearIssuesChapter / The Work content (reference lines 753-826).
					$fact(
						'cl-acme-work-1',
						'workItems[0]',
						'Finalize renewal owner map. Name the legal and launch owners before Jen forwards the executive readout. (Target: Jun 3 · 1 of 2 milestones)',
						'likely_current'
					),
					$fact(
						'cl-acme-work-2',
						'workItems[1]',
						'Send revised pricing appendix to procurement.',
						'use_with_caution'
					),
				],
				'next_cursor'  => null,
				'total_hint'   => 21,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'health_story'         => [
				'summary'         => 'Procurement joined the renewal thread and asked for security evidence before approving the extension.',
				'trust_band'      => 'use_with_caution',
				'aggregate_score' => 71,
			],
			'metadata_proposals'   => self::empty_paginated(),
			'open_loops'           => [
				'items'        => [
					self::open_loop_item( 'sec-questionnaire-owner', 'Security questionnaire needs an owner before renewal review', 'overdue', '~25m' ),
					self::open_loop_item( 'send-pricing-appendix', 'Send revised pricing appendix', 'overdue', '~10m' ),
					self::open_loop_item( 'confirm-attendees', 'Confirm attendee list for renewal call', 'done', '' ),
				],
				'next_cursor'  => null,
				'total_hint'   => 3,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'touchpoints'          => [
				'items'        => [
					self::touchpoint_item( 'tp-acme-renewal-now', 'Acme Corp renewal checkpoint', '2026-05-22T10:00:00Z', 'present', 6 ),
					self::touchpoint_item( 'tp-acme-prep-apr21', 'Acme Corp commercial prep', '2026-04-21T15:00:00Z', 'past', 4 ),
					self::touchpoint_item( 'tp-acme-eom-review', 'Acme Corp end-of-month review', '2026-03-28T14:00:00Z', 'past', 5 ),
				],
				'next_cursor'  => null,
				'total_hint'   => 3,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'threads'              => self::empty_paginated(),
			'record_entries'       => [
				'items'        => [
					[
						'claimId'      => 'cl-acme-record-may4',
						'fieldPath'    => 'recordEntries[0]',
						'renderedText' => 'Acme Corp renewal checkpoint — Customer.',
						'trustBand'    => 'likely_current',
						'sourceAsof'   => '2026-05-04T09:00:00Z',
						'dataSource'   => 'Meeting',
						'subjectRef'   => $subject_ref,
						'sensitivity'  => [ 'kind' => 'normal' ],
						'entryKind'    => 'meeting',
						'entryDate'    => 'May 4',
					],
					[
						'claimId'      => 'cl-acme-record-may2',
						'fieldPath'    => 'recordEntries[1]',
						'renderedText' => 'Jen asked for a single legal owner before forwarding the package.',
						'trustBand'    => 'likely_current',
						'sourceAsof'   => '2026-05-02T11:00:00Z',
						'dataSource'   => 'jen.park@acme.example',
						'subjectRef'   => $subject_ref,
						'sensitivity'  => [ 'kind' => 'normal' ],
						'entryKind'    => 'email',
						'entryDate'    => 'May 2',
					],
					[
						'claimId'      => 'cl-acme-record-apr29',
						'fieldPath'    => 'recordEntries[2]',
						'renderedText' => 'Renewal packet framing — anchor the next update around owner, date, and decision requested.',
						'trustBand'    => 'likely_current',
						'sourceAsof'   => '2026-04-29T14:00:00Z',
						'dataSource'   => 'Note',
						'subjectRef'   => $subject_ref,
						'sensitivity'  => [ 'kind' => 'normal' ],
						'entryKind'    => 'note',
						'entryDate'    => 'Apr 29',
					],
					[
						'claimId'      => 'cl-acme-record-apr25',
						'fieldPath'    => 'recordEntries[3]',
						'renderedText' => 'Milestone completed: Admin workflow rehearsal — auto-completed by meeting evidence.',
						'trustBand'    => 'likely_current',
						'sourceAsof'   => '2026-04-25T16:00:00Z',
						'dataSource'   => 'Auto',
						'subjectRef'   => $subject_ref,
						'sensitivity'  => [ 'kind' => 'normal' ],
						'entryKind'    => 'value',
						'entryDate'    => 'Apr 25',
					],
				],
				'next_cursor'  => null,
				'total_hint'   => 4,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'trust'                => [
				'aggregate_band'           => 'use_with_caution',
				'likely_current_count'     => 18,
				'use_with_caution_count'   => 3,
				'needs_verification_count' => 0,
			],
			'intelligence_quality' => [
				'level' => 'fresh',
			],
			'provenance'           => self::provenance_now(),
			'sensitivity'          => [ 'kind' => 'normal' ],
		];
	}

	/**
	 * Build a project envelope.
	 *
	 * @param string $entity_id Entity ID.
	 * @return array<string,mixed>
	 */
	private static function project_envelope( string $entity_id ): array {
		$id = '' === $entity_id ? 'beta-migration' : $entity_id;
		return [
			'schemaVersion'      => 1,
			'envelopeRenderId'   => 'mock-env-' . $id,
			'subject'            => [
				'kind' => 'project',
				'id'   => $id,
				'name' => 'Beta Migration',
			],
			'sections'           => self::sections_all_present(),
			'facts'              => [
				'items'        => [
					[
						'key'        => 'status',
						'value'      => 'active',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'trajectory',
						'value'      => 'improving',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'parent_account',
						'value'      => 'Acme Corp',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'milestone',
						'value'      => 'Q2 Launch',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'target_date',
						'value'      => '2026-06-30',
						'trust_band' => 'likely_current',
					],
				],
				'next_cursor'  => null,
				'total_hint'   => 5,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'health_story'       => [
				'summary'         => 'Beta migration thread is warming up again after two quiet weeks; champion forwarded the migration plan.',
				'trust_band'      => 'likely_current',
				'aggregate_score' => 78,
			],
			'metadata_proposals' => self::empty_paginated(),
			'open_loops'         => [
				'items'        => [
					self::open_loop_item( 'audit-log-export-owner', 'Confirm audit-log export owner', 'today', '~15m' ),
				],
				'next_cursor'  => null,
				'total_hint'   => 1,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'touchpoints'        => self::empty_paginated(),
			'threads'            => self::empty_paginated(),
			'record_entries'     => self::empty_paginated(),
			'trust'              => [
				'aggregate_band'           => 'likely_current',
				'likely_current_count'     => 5,
				'use_with_caution_count'   => 0,
				'needs_verification_count' => 0,
			],
			'provenance'         => self::provenance_now(),
			'sensitivity'        => [ 'kind' => 'normal' ],
		];
	}

	/**
	 * Build a person envelope.
	 *
	 * @param string $entity_id Entity ID.
	 * @return array<string,mixed>
	 */
	private static function person_envelope( string $entity_id ): array {
		$id = '' === $entity_id ? 'priya-raman' : $entity_id;
		return [
			'schemaVersion'      => 1,
			'envelopeRenderId'   => 'mock-env-' . $id,
			'subject'            => [
				'kind' => 'person',
				'id'   => $id,
				'name' => 'Priya Raman',
			],
			'sections'           => self::sections_all_present(),
			'facts'              => [
				'items'        => [
					[
						'key'        => 'display_name',
						'value'      => 'Priya Raman',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'role',
						'value'      => 'Director of Engineering',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'organization',
						'value'      => 'Acme Corp',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'email',
						'value'      => 'priya@subsidiary.com',
						'trust_band' => 'likely_current',
					],
					[
						'key'        => 'last_seen',
						'value'      => '2026-05-22T10:00:00Z',
						'trust_band' => 'likely_current',
					],
				],
				'next_cursor'  => null,
				'total_hint'   => 5,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'health_story'       => null,
			'metadata_proposals' => self::empty_paginated(),
			'open_loops'         => self::empty_paginated(),
			'touchpoints'        => self::empty_paginated(),
			'threads'            => self::empty_paginated(),
			'record_entries'     => self::empty_paginated(),
			'trust'              => [
				'aggregate_band'           => 'likely_current',
				'likely_current_count'     => 5,
				'use_with_caution_count'   => 0,
				'needs_verification_count' => 0,
			],
			'provenance'         => self::provenance_now(),
			'sensitivity'        => [ 'kind' => 'normal' ],
		];
	}

	/**
	 * Build an empty envelope.
	 *
	 * @param string $entity_type Entity type.
	 * @param string $entity_id   Entity ID.
	 * @return array<string,mixed>
	 */
	private static function empty_envelope( string $entity_type, string $entity_id ): array {
		return [
			'schemaVersion'      => 1,
			'envelopeRenderId'   => 'mock-env-empty-' . $entity_type . '-' . $entity_id,
			'subject'            => [
				'kind' => $entity_type,
				'id'   => $entity_id,
				'name' => sprintf( 'Unknown %s', $entity_type ),
			],
			'sections'           => self::sections_all_empty( 'unknown_entity' ),
			'facts'              => self::empty_paginated(),
			'health_story'       => null,
			'metadata_proposals' => self::empty_paginated(),
			'open_loops'         => self::empty_paginated(),
			'touchpoints'        => self::empty_paginated(),
			'threads'            => self::empty_paginated(),
			'record_entries'     => self::empty_paginated(),
			'trust'              => [
				'aggregate_band'           => 'unscored',
				'likely_current_count'     => 0,
				'use_with_caution_count'   => 0,
				'needs_verification_count' => 0,
			],
			'provenance'         => self::provenance_now(),
			'sensitivity'        => [ 'kind' => 'normal' ],
		];
	}

	/**
	 * Get meeting prep status.
	 *
	 * @param array<string,mixed> $payload Ability payload.
	 * @return array<string,mixed>
	 */
	public static function meeting_prep_status( array $payload ): array {
		$meeting_id = isset( $payload['meeting_id'] ) ? (string) $payload['meeting_id'] : 'mtg-acme-renewal-checkpoint';
		return [
			'meeting_id'         => $meeting_id,
			'event_id'           => 'evt-' . $meeting_id,
			'linked_entity_type' => 'account',
			'linked_entity_id'   => 'acme-corp',
			'status'             => 'ready',
			'blocking_reason'    => null,
			'stale_reason'       => null,
			'last_prepared_at'   => '2026-05-22T09:42:00Z',
			'source_asof_inputs' => [
				[
					'source' => 'gmail',
					'as_of'  => '2026-05-22T09:42:00Z',
				],
				[
					'source' => 'calendar',
					'as_of'  => '2026-05-22T09:30:00Z',
				],
			],
		];
	}

	/**
	 * Get daily briefing.
	 *
	 * @param array<string,mixed> $payload Ability payload.
	 * @return array<string,mixed>
	 */
	public static function daily_briefing( array $payload ): array {
		return [
			'schemaVersion'      => 1,
			'date'               => '2026-05-22',
			'state'              => [
				'availability' => [ 'kind' => 'available' ],
				'freshness'    => [ 'kind' => 'fresh' ],
				'integrity'    => [ 'kind' => 'clean' ],
				'advisories'   => [],
			],
			'current_meeting'    => null,
			'next_meeting'       => self::meeting_brief_ref( 'mtg-acme-renewal-checkpoint', 'Acme renewal call', '10:00 AM', '45m', 'customer' ),
			'upcoming_meetings'  => [
				'items'        => [
					self::meeting_brief_ref( 'mtg-product-strategy', 'Product strategy sync', '1:30 PM', '25m', 'internal' ),
					self::meeting_brief_ref( 'mtg-globex-vendor-review', 'Globex vendor review', '3:00 PM', '30m', 'cancelled' ),
				],
				'next_cursor'  => null,
				'total_hint'   => 2,
				'cursor_state' => [ 'kind' => 'stable' ],
			],
			'candidate_set'      => [ 'meeting_ids' => [] ],
			'watch_proposals'    => [],
			'trust_summary'      => [
				'aggregate_band'           => 'likely_current',
				'likely_current_count'     => 12,
				'use_with_caution_count'   => 2,
				'needs_verification_count' => 0,
			],
			'provenance'         => self::provenance_now(),
			'sensitivity'        => [ 'kind' => 'normal' ],
			'source_asof_inputs' => [
				[
					'source' => 'gmail',
					'as_of'  => '2026-05-22T09:42:00Z',
				],
				[
					'source' => 'calendar',
					'as_of'  => '2026-05-22T09:30:00Z',
				],
			],
		];
	}

	/**
	 * Get claim receipt.
	 *
	 * @param array<string,mixed> $payload Ability payload.
	 * @return array<string,mixed>
	 */
	public static function claim_receipt( array $payload ): array {
		return [
			'rows'     => [],
			'audience' => 'meeting',
		];
	}

	/**
	 * List accounts.
	 *
	 * @param array<string,mixed> $payload Ability payload.
	 * @return array<string,mixed>
	 */
	public static function list_accounts( array $payload ): array {
		return [
			'items'                 => [
				[
					'account_id'         => 'acme-corp',
					'name'               => 'Acme Corp',
					'status'             => 'renewing',
					'health_band'        => 'use_with_caution',
					'last_touchpoint_at' => '2026-05-22T10:00:00Z',
					'open_loops_count'   => 3,
				],
				[
					'account_id'         => 'globex-holdings',
					'name'               => 'Globex Holdings',
					'status'             => 'paused',
					'health_band'        => 'needs_verification',
					'last_touchpoint_at' => '2026-05-18T14:00:00Z',
					'open_loops_count'   => 1,
				],
				[
					'account_id'         => 'northstar-inc',
					'name'               => 'Northstar Inc',
					'status'             => 'active',
					'health_band'        => 'likely_current',
					'last_touchpoint_at' => '2026-05-21T16:30:00Z',
					'open_loops_count'   => 0,
				],
			],
			'total_after_filter'    => 3,
			'data_shifted_advisory' => null,
		];
	}

	/**
	 * List open loops.
	 *
	 * @param array<string,mixed> $payload Ability payload.
	 * @return array<string,mixed>
	 */
	public static function list_open_loops( array $payload ): array {
		return [
			'claims' => [
				[
					'claim_id'     => 'cl-msa-redlines',
					'subject_kind' => 'account',
					'subject_id'   => 'acme-corp',
					'headline'     => 'Send Acme Corp final MSA redlines',
					'urgency'      => 'overdue',
				],
				[
					'claim_id'     => 'cl-sponsor-coverage',
					'subject_kind' => 'account',
					'subject_id'   => 'acme-corp',
					'headline'     => 'Confirm sponsor coverage',
					'urgency'      => 'today',
				],
			],
		];
	}

	/**
	 * Build all-present sections.
	 *
	 * @return array<string,array<string,mixed>>
	 */
	private static function sections_all_present(): array {
		$sections = [];
		foreach ( [ 'facts', 'health', 'metadata_proposals', 'open_loops', 'touchpoints', 'threads', 'record' ] as $section ) {
			$sections[ $section ] = [
				'kind'       => 'present',
				'item_count' => 1,
			];
		}
		return $sections;
	}

	/**
	 * Build all-empty sections.
	 *
	 * @param string $reason Empty reason.
	 * @return array<string,array<string,mixed>>
	 */
	private static function sections_all_empty( string $reason ): array {
		$sections = [];
		foreach ( [ 'facts', 'health', 'metadata_proposals', 'open_loops', 'touchpoints', 'threads', 'record' ] as $section ) {
			$sections[ $section ] = [
				'kind'   => 'empty',
				'reason' => $reason,
			];
		}
		return $sections;
	}

	/**
	 * Build an empty paginated response.
	 *
	 * @return array<string,mixed>
	 */
	private static function empty_paginated(): array {
		return [
			'items'        => [],
			'next_cursor'  => null,
			'total_hint'   => 0,
			'cursor_state' => [ 'kind' => 'stable' ],
		];
	}

	/**
	 * Build an open loop item.
	 *
	 * @param string $id       Open loop ID.
	 * @param string $headline Open loop headline.
	 * @param string $urgency  Open loop urgency.
	 * @param string $estimate Open loop estimate.
	 * @return array<string,mixed>
	 */
	private static function open_loop_item( string $id, string $headline, string $urgency, string $estimate ): array {
		return [
			'claim_id'     => 'cl-' . $id,
			'headline'     => $headline,
			'urgency'      => $urgency,
			'estimate'     => $estimate,
			'subject_kind' => 'account',
			'subject_id'   => 'acme-corp',
			'receipt'      => [
				'rows'     => [],
				'audience' => 'open_loops',
			],
		];
	}

	/**
	 * Build a touchpoint item.
	 *
	 * @param string $id             Touchpoint ID.
	 * @param string $title          Touchpoint title.
	 * @param string $when           Touchpoint time.
	 * @param string $side           Touchpoint side.
	 * @param int    $attendee_count Attendee count.
	 * @return array<string,mixed>
	 */
	private static function touchpoint_item( string $id, string $title, string $when, string $side, int $attendee_count ): array {
		return [
			'touchpoint_id'    => 'tp-' . $id,
			'meeting_id'       => 'mtg-' . $id,
			'title'            => $title,
			'when'             => $when,
			'side'             => $side,
			'attendee_count'   => $attendee_count,
			'inclusion_reason' => 'subject_match',
		];
	}

	/**
	 * Build a meeting brief reference.
	 *
	 * @param string $meeting_id Meeting ID.
	 * @param string $title      Meeting title.
	 * @param string $time_local Local time.
	 * @param string $duration   Meeting duration.
	 * @param string $kind       Meeting kind.
	 * @return array<string,mixed>
	 */
	private static function meeting_brief_ref( string $meeting_id, string $title, string $time_local, string $duration, string $kind ): array {
		return [
			'meeting_id'      => $meeting_id,
			'title'           => $title,
			'time_local'      => $time_local,
			'duration'        => $duration,
			'meeting_kind'    => $kind,
			'attendee_count'  => 6,
			'primary_account' => 'Acme Corp',
		];
	}

	/**
	 * Build meeting attendees.
	 *
	 * @return array<int,array<string,mixed>>
	 */
	private static function meeting_attendees(): array {
		return [
			[
				'person_id'          => 'jen-park',
				'display_name'       => 'Jen Park',
				'avatar_initial'     => 'J',
				'avatar_style'       => 'default',
				'role'               => 'VP Customer Operations',
				'organization'       => 'Acme Corp',
				'temperature'        => 'warm',
				'engagement'         => 'champion',
				'assessment'         => 'Jen will sponsor the executive readout if the redline owner is explicit and the Q2 Launch value story stays concrete.',
				'meeting_count'      => 6,
				'last_seen_label'    => 'Last seen yesterday',
				'tooltip_assessment' => 'Executive sponsor; wants the commercial story before legal review.',
			],
			[
				'person_id'       => 'dan-mitchell',
				'display_name'    => 'Dan Mitchell',
				'avatar_initial'  => 'D',
				'avatar_style'    => 'cold',
				'role'            => 'Procurement Lead',
				'organization'    => 'Acme Corp',
				'temperature'     => 'cold',
				'engagement'      => 'detractor',
				'assessment'      => 'Dan needs a bounded owner and timeline before he will stop reopening MSA language.',
				'meeting_count'   => 3,
				'last_seen_label' => 'Last seen 2 weeks ago',
			],
			[
				'person_id'       => 'sara-wu',
				'display_name'    => 'Sara Wu',
				'avatar_initial'  => 'S',
				'avatar_style'    => 'new',
				'role'            => 'Technical Approver',
				'organization'    => 'Acme Corp',
				'temperature'     => 'cool',
				'engagement'      => 'new_contact',
				'assessment'      => 'Sara wants technical risk summarized separately from the commercial story.',
				'meeting_count'   => 1,
				'last_seen_label' => 'New contact',
			],
			[
				'person_id'       => 'priya-raman',
				'display_name'    => 'Priya Raman',
				'avatar_initial'  => 'P',
				'avatar_style'    => 'default',
				'role'            => 'Director of Engineering',
				'organization'    => 'Acme Corp',
				'temperature'     => 'warm',
				'engagement'      => 'supporter',
				'assessment'      => 'Priya is the day-to-day partner; will reinforce technical commitments if framed concretely.',
				'meeting_count'   => 12,
				'last_seen_label' => 'Last seen 3 days ago',
			],
			[
				'person_id'       => 'owen-carter',
				'display_name'    => 'Owen Carter',
				'avatar_initial'  => 'O',
				'avatar_style'    => 'default',
				'role'            => 'Legal Counsel',
				'organization'    => 'Acme Corp',
				'temperature'     => 'cool',
				'engagement'      => 'tentative',
				'assessment'      => 'Tentative attendee; only joins for procurement discussion.',
				'meeting_count'   => 2,
				'last_seen_label' => 'Last seen 6 days ago',
			],
			[
				'person_id'       => 'james-giroux',
				'display_name'    => 'James Giroux',
				'avatar_initial'  => 'J',
				'avatar_style'    => 'self',
				'role'            => 'Customer Success Lead',
				'organization'    => 'DailyOS',
				'temperature'     => 'self',
				'engagement'      => 'self',
				'assessment'      => '',
				'meeting_count'   => 24,
				'last_seen_label' => 'You',
			],
		];
	}

	/**
	 * Build meeting risks.
	 *
	 * @return array<int,array<string,mixed>>
	 */
	private static function meeting_risks(): array {
		return [
			[
				'rank'     => 'featured',
				'urgency'  => 'high',
				'text'     => 'If the MSA redlines leave this meeting without one named owner, procurement will treat the renewal as slipping and reopen pricing pressure.',
				'claim_id' => 'cl-risk-msa-redlines',
			],
			[
				'rank'     => 'subordinate',
				'urgency'  => 'high',
				'text'     => 'Dan has not accepted audit-log ownership; if you leave it implied, the Q2 Launch plan will still have a visible gap.',
				'claim_id' => 'cl-risk-audit-log-owner',
			],
			[
				'rank'     => 'subordinate',
				'urgency'  => 'medium',
				'text'     => 'Sara is new to the renewal path; overloading her with commercial detail before the technical summary could dilute the architecture signal.',
				'claim_id' => 'cl-risk-sara-onboarding',
			],
		];
	}

	/**
	 * Build meeting recent wins.
	 *
	 * @return array<int,array<string,mixed>>
	 */
	private static function meeting_recent_wins(): array {
		return [
			[
				'text'     => 'Helpline Rollout rehearsal landed cleanly with support; Acme Corp no longer sees launch readiness as the blocker.',
				'claim_id' => 'cl-win-helpline-rollout',
			],
			[
				'text'     => 'Jen volunteered to sponsor the executive readout if the legal owner and launch owner are explicit by end of day.',
				'claim_id' => 'cl-win-jen-sponsor',
			],
		];
	}

	/**
	 * Build meeting readiness items.
	 *
	 * @return array<int,array<string,mixed>>
	 */
	private static function meeting_readiness_items(): array {
		return [
			[
				'text'     => 'Have the MSA redline summary ready for Jen, with the two clauses legal still owns.',
				'dot_tone' => 'turmeric_muted',
			],
			[
				'text'     => 'Bring the Q2 Launch dependency map so Dan can confirm the audit-log owner live.',
				'dot_tone' => 'turmeric_muted',
			],
			[
				'text'     => 'Ask Sara whether the technical risk summary should go to the exec readout or stay in the renewal thread.',
				'dot_tone' => 'turmeric_muted',
			],
		];
	}

	/**
	 * Build meeting recommended actions.
	 *
	 * @return array<int,array<string,mixed>>
	 */
	private static function meeting_recommended_actions(): array {
		return [
			[
				'action_id' => 'act-confirm-msa-owner',
				'headline'  => 'Confirm the MSA redline owner before procurement re-engages.',
				'urgency'   => 'overdue',
				'context'   => '~15m · Acme Corp',
				'why'       => 'Blocks the legal close-out path Jen named.',
			],
			[
				'action_id' => 'act-publish-launch-deps',
				'headline'  => 'Publish Q2 Launch dependency map with named owners.',
				'urgency'   => 'today',
				'context'   => '~20m · Beta Migration',
				'why'       => 'Lets Dan accept audit-log ownership live.',
			],
		];
	}

	/**
	 * Build meeting post intelligence.
	 *
	 * @return array<string,mixed>
	 */
	private static function meeting_post_intel(): array {
		// Post-meeting intelligence — replaces flat outcomes when available.
		// Referenced by reference HTML lines 59-160 (PostMeetingIntelligence_*).
		return [
			'summary'      => 'Acme validated the Q2 Launch path and narrowed the renewal risk to one legal owner and one launch dependency. Jen Park agreed to sponsor the executive follow-up once Dan Mitchell confirms the audit-log export owner.',
			'thread_items' => [
				[
					'kind'     => 'confirmed',
					'headline' => 'Finalize support workflow rehearsal',
					'detail'   => 'closed since the last meeting',
				],
				[
					'kind'     => 'open',
					'headline' => 'Review final MSA redlines',
					'detail'   => 'due Apr 23',
				],
				[
					'kind'     => 'neutral',
					'headline' => 'Health moved from 74 to 82',
					'detail'   => '',
				],
				[
					'kind'     => 'new_face',
					'headline' => 'Sara Wu',
					'detail'   => 'new attendee',
				],
			],
			'predictions'  => [
				'risks'         => [
					[
						'matched'    => true,
						'prediction' => 'MSA redlines could stall procurement',
						'reality'    => 'Jen asked for one legal owner before procurement sees the package.',
					],
					[
						'matched'    => false,
						'prediction' => 'Pricing pressure would resurface',
						'reality'    => '',
					],
				],
				'opportunities' => [
					[
						'matched'    => true,
						'prediction' => 'Sara would surface technical depth on the audit-log path',
						'reality'    => 'Sara confirmed the export contract is the binding architectural decision.',
					],
				],
			],
		];
	}

	/**
	 * Build a canned projection.
	 *
	 * Used by the resurrected `dailyos/account-overview` block for
	 * compare-against-known-good debugging.
	 *
	 * @param string $key Projection key.
	 * @return array<string,mixed>|null
	 */
	public static function canned_projection( string $key ): ?array {
		$recent_iso = gmdate( 'c', time() - 3600 );
		$stale_iso  = gmdate( 'c', time() - 7 * 86400 );

		$blocks_by_key = [
			'account-overview'             => [
				[
					'selected_known_type_id' => 'dailyos/account-overview-summary',
					'trust_band'             => 'likely_current',
					'payload'                => [
						'title' => 'Acme Corp',
						'text'  => 'Customer account currently renewing. Procurement engaged the renewal thread and asked for security evidence before approving the extension. Champion remains engaged; technical risk narrowed to one legal owner and one launch dependency.',
					],
				],
				[
					'selected_known_type_id' => 'dailyos/action-list',
					'trust_band'             => 'likely_current',
					'payload'                => [
						'title' => 'Open actions',
						'items' => [
							'Send Acme Corp final MSA redlines to Jen Park',
							'Confirm sponsor coverage before legal review',
							'Publish Q2 Launch dependency map with named owners',
						],
					],
				],
			],
			'entity-chip-account'          => [
				[
					'selected_known_type_id' => 'dailyos/entity-chip',
					'payload'                => [
						'entity_type' => 'account',
						'text'        => 'Acme Corp',
					],
				],
			],
			'entity-chip-project'          => [
				[
					'selected_known_type_id' => 'dailyos/entity-chip',
					'payload'                => [
						'entity_type' => 'project',
						'text'        => 'Beta Migration',
					],
				],
			],
			'entity-chip-person'           => [
				[
					'selected_known_type_id' => 'dailyos/entity-chip',
					'payload'                => [
						'entity_type' => 'person',
						'text'        => 'Priya Raman',
					],
				],
			],
			'health-badge-compact-green'   => [
				[
					'selected_known_type_id' => 'dailyos/health-badge',
					'payload'                => [
						'score'          => 82,
						'band'           => 'green',
						'size'           => 'compact',
						'sufficientData' => true,
						'showScore'      => true,
					],
				],
			],
			'health-badge-standard-yellow' => [
				[
					'selected_known_type_id' => 'dailyos/health-badge',
					'payload'                => [
						'score'          => 71,
						'band'           => 'yellow',
						'size'           => 'standard',
						'sufficientData' => true,
						'showScore'      => true,
						'trend'          => [ 'direction' => 'declining' ],
					],
				],
			],
			'health-badge-hero-red'        => [
				[
					'selected_known_type_id' => 'dailyos/health-badge',
					'payload'                => [
						'score'          => 41,
						'band'           => 'red',
						'size'           => 'hero',
						'sufficientData' => true,
						'showScore'      => true,
						'trend'          => [ 'direction' => 'declining' ],
					],
				],
			],
			'trust-band-current'           => [
				[
					'selected_known_type_id' => 'dailyos/trust-band-badge',
					'payload'                => [ 'band' => 'likely_current' ],
				],
			],
			'trust-band-caution'           => [
				[
					'selected_known_type_id' => 'dailyos/trust-band-badge',
					'payload'                => [ 'band' => 'use_with_caution' ],
				],
			],
			'freshness-recent'             => [
				[
					'selected_known_type_id' => 'dailyos/freshness-indicator',
					'payload'                => [
						'at'      => $recent_iso,
						'format'  => 'relative',
						'variant' => 'inline',
						'verb'    => 'Updated',
					],
				],
			],
			'freshness-stale'              => [
				[
					'selected_known_type_id' => 'dailyos/freshness-indicator',
					'payload'                => [
						'at'      => $stale_iso,
						'format'  => 'relative',
						'variant' => 'inline',
						'verb'    => 'Updated',
					],
				],
			],
		];

		if ( ! isset( $blocks_by_key[ $key ] ) ) {
			return null;
		}
		return [
			'blocks' => $blocks_by_key[ $key ],
		];
	}

	/**
	 * Build current provenance.
	 *
	 * @return array<string,mixed>
	 */
	private static function provenance_now(): array {
		return [
			'source_asof' => '2026-05-22T09:42:00Z',
			'sources'     => [
				[
					'source' => 'gmail',
					'as_of'  => '2026-05-22T09:42:00Z',
				],
				[
					'source' => 'calendar',
					'as_of'  => '2026-05-22T09:30:00Z',
				],
			],
		];
	}
}
