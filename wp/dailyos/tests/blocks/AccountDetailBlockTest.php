<?php
/**
 * W2 L1 DOS-462 account-detail composite block tests.
 *
 * Covers the outer dailyos/account-detail block + a representative subset
 * of inner blocks per L0-packet-W2-entity-surfaces.md V1.2.1 §5.1 + AC-462.
 *
 * Test contracts:
 *  - outer block invokes get_entity_intelligence ONCE per render with the
 *    3-arg invoke_ability signature (AC-W1.9 / check_w1_consumer_skeleton).
 *  - outer block.json declares apiVersion 3, parent null, providesContext
 *    {entityType, entityId, envelopeHandle}, templateLock false, 24-entry
 *    default template (AC-462.2).
 *  - every inner block.json registers apiVersion 3, no parent,
 *    usesContext envelope binding (AC-462.2).
 *  - empty envelope renders the dailyos-empty-chip with data-empty-reason
 *    per V1.1 §10 invariant (AC-462.6 visible-state matrix).
 *  - inner blocks consume the cached envelope via dailyos_resolve_envelope
 *    and do NOT call get_entity_intelligence a second time when the
 *    handle is present (envelopeHandle single-fetch contract).
 *
 * No customer data in fixtures — generic IDs only per CLAUDE.md.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

require_once __DIR__ . '/../../blocks/_shared/envelope/envelope-resolver.php';
require_once __DIR__ . '/../../blocks/account-detail/render-functions.php';
require_once __DIR__ . '/../../blocks/account-detail/inner/account-hero/render-functions.php';

/**
 * @covers dailyos_account_detail_render
 * @covers dailyos_resolve_envelope
 * @covers dailyos_envelope_section
 * @covers dailyos_empty_chip
 */
final class DailyOS_AccountDetailBlockTest extends TestCase {

	/**
	 * Resets test globals before each test.
	 */
	protected function setUp(): void {
		parent::setUp();
		if ( function_exists( 'dailyos_test_reset_globals' ) ) {
			dailyos_test_reset_globals();
		}
		unset( $GLOBALS['dailyos_test_filters']['dailyos_runtime_client_for_block'] );
		unset( $GLOBALS['dailyos_envelope_handle_for_request'] );
		// Reset the request-scoped envelope cache by re-requiring shouldn't
		// re-create static storage, so we mutate via the public putter on a
		// dummy key to no-op when needed. The static scope is per-process
		// so PHPUnit's process isolation already handles this.
	}

	// ---- outer block: contract ------------------------------------------

	/**
	 * Outer block.json declares the W2 contract.
	 */
	public function test_outer_block_json_declares_w2_contract(): void {
		$block_json = json_decode(
			(string) file_get_contents( __DIR__ . '/../../blocks/account-detail/block.json' ),
			true
		);
		$this->assertIsArray( $block_json );
		$this->assertSame( 'dailyos/account-detail', $block_json['name'] );
		$this->assertSame( 3, $block_json['apiVersion'] );
		$this->assertSame( 'dailyos', $block_json['category'] );
		$this->assertNull( $block_json['parent'] );
		$this->assertSame( 'file:./render.php', $block_json['render'] );
		$this->assertFalse( $block_json['templateLock'] );
		$this->assertCount( 24, $block_json['template'] );
		$this->assertSame( [ 'dailyos/account-hero' ], $block_json['template'][0] );
		$this->assertSame( [ 'dailyos/finis-marker' ], $block_json['template'][23] );

		$context = $block_json['providesContext'];
		$this->assertArrayHasKey( 'dailyos/entityType', $context );
		$this->assertArrayHasKey( 'dailyos/entityId', $context );
		$this->assertArrayHasKey( 'dailyos/envelopeHandle', $context );
	}

	// ---- outer block: render --------------------------------------------

	/**
	 * Outer render returns the empty-chip when no account_id is supplied.
	 */
	public function test_render_returns_empty_chip_on_missing_account_id(): void {
		$html = dailyos_account_detail_render( [] );
		$this->assertStringContainsString( 'dailyos-empty-chip', $html );
		$this->assertStringContainsString( 'data-empty-reason="no_account_id"', $html );
	}

	/**
	 * Outer render returns the empty-chip when no runtime client is bound.
	 */
	public function test_render_returns_empty_chip_when_runtime_unavailable(): void {
		$html = dailyos_account_detail_render( [ 'account_id' => 'acct-test-001' ] );
		$this->assertStringContainsString( 'dailyos-empty-chip', $html );
		$this->assertStringContainsString( 'data-empty-reason="runtime_unavailable"', $html );
	}

	/**
	 * Runtime error envelopes are surfaced as errors, not cached as fake
	 * entity envelopes.
	 */
	public function test_render_returns_empty_chip_when_runtime_returns_error_envelope(): void {
		$client = $this->fake_runtime_client_with_envelope(
			[
				'ok'    => false,
				'error' => [
					'code'    => 'runtime_request_failed',
					'message' => 'DailyOS runtime request failed.',
				],
			]
		);
		$this->register_runtime_client_filter( $client );

		$html = dailyos_account_detail_render( [ 'account_id' => 'acct-test-001' ] );

		$this->assertStringContainsString( 'dailyos-empty-chip', $html );
		$this->assertStringContainsString( 'data-empty-reason="runtime_request_failed"', $html );
		$this->assertStringNotContainsString( 'data-dailyos-envelope-handle=', $html );
	}

	/**
	 * Outer render emits the envelope handle into the wrapper attrs and
	 * routes inner content through do_blocks. Producer is invoked exactly
	 * once with the 3-arg signature.
	 */
	public function test_render_invokes_get_entity_intelligence_once_with_three_args(): void {
		$client = $this->fake_runtime_client_with_envelope( $this->envelope_response_present() );
		$this->register_runtime_client_filter( $client );

		$html = dailyos_account_detail_render(
			[ 'account_id' => 'acct-test-001' ],
			''
		);

		$this->assertSame( 1, $client->calls, 'producer invoked exactly once per outer render' );
		$this->assertSame( 'get_entity_intelligence', $client->requests[0]['ability'] );
		$this->assertSame( 'account', $client->requests[0]['payload']['entity_type'] );
		$this->assertSame( 'acct-test-001', $client->requests[0]['payload']['entity_id'] );
		$this->assertIsArray( $client->requests[0]['scope_set'] );
		$this->assertStringContainsString( 'data-dailyos-envelope-handle=', $html );
		$this->assertStringContainsString( 'wp-block-dailyos-account-detail', $html );
		$this->assertStringContainsString( 'dailyos-inner-blocks-slot', $html );
	}

	/**
	 * Outer render emits a wrapper with NO inline style when the subject
	 * carries no chrome tint — only --dailyos-* custom properties are
	 * allowed (check_no_inline_style_exception gate).
	 */
	public function test_render_omits_inline_style_when_no_tint(): void {
		$client = $this->fake_runtime_client_with_envelope( $this->envelope_response_present() );
		$this->register_runtime_client_filter( $client );

		$html = dailyos_account_detail_render(
			[ 'account_id' => 'acct-test-001' ],
			''
		);
		// No inline style at all when no allowlisted custom property.
		$this->assertStringNotContainsString( 'style=', $html );
	}

	/**
	 * Outer render forwards a --dailyos-account-tint custom property when
	 * the subject carries one — allowlisted form.
	 */
	public function test_render_emits_allowlisted_custom_property_tint(): void {
		$envelope                                      = $this->envelope_response_present();
		$envelope['envelope']['subject']['chromeTint'] = 'var(--dailyos-tint-cobalt)';
		$client                                        = $this->fake_runtime_client_with_envelope( $envelope );
		$this->register_runtime_client_filter( $client );

		$html = dailyos_account_detail_render(
			[ 'account_id' => 'acct-test-001' ],
			''
		);
		$this->assertMatchesRegularExpression(
			'/style="--dailyos-account-tint: var\(--dailyos-tint-cobalt\)"/',
			$html
		);
	}

	// ---- 24 inner blocks: block.json contract ---------------------------

	/**
	 * Every inner block declares the V1.2.1 §5.1 inner-block contract.
	 * apiVersion 3, no parent, usesContext for envelope binding.
	 */
	public function test_all_inner_blocks_declare_v1_2_1_contract(): void {
		$inner_dir = __DIR__ . '/../../blocks/account-detail/inner';
		$dirs      = glob( $inner_dir . '/*', GLOB_ONLYDIR );
		$this->assertCount( 24, $dirs, '24 inner blocks per L0 packet V1.2.1 §5.1' );
		foreach ( $dirs as $dir ) {
			$json_path = $dir . '/block.json';
			$this->assertFileExists( $json_path );
			$json = json_decode( (string) file_get_contents( $json_path ), true );
			$this->assertSame( 3, $json['apiVersion'], $json_path );
			$this->assertSame( 'dailyos', $json['category'], $json_path );
			$this->assertArrayNotHasKey( 'parent', $json, $json_path . ' inner blocks are inserter-global per ADR-0129 §2' );
			$this->assertContains( 'dailyos/envelopeHandle', $json['usesContext'], $json_path );
			$this->assertContains( 'dailyos/entityId', $json['usesContext'], $json_path );
			$this->assertContains( 'dailyos/entityType', $json['usesContext'], $json_path );
			$this->assertSame( 'file:./render.php', $json['render'], $json_path );
		}
	}

	/**
	 * Block-name collision guard: the 4 feed-shape inner blocks that share
	 * a slug with person-detail's top-level (de-facto person-detail-inner)
	 * blocks MUST be surface-prefixed. register_block_type is first-wins,
	 * so without the prefix the top-level wins, the account-detail inner
	 * block silently fails to register, and the rendered surface emits
	 * the wrong empty-state reason (`no_envelope` from the person-detail
	 * renderer that calls a person-detail-specific envelope store).
	 */
	public function test_collision_slugs_are_surface_prefixed(): void {
		$collision_slugs = [ 'recommended-actions', 'touchpoints-feed', 'open-loops-feed', 'unified-timeline' ];
		foreach ( $collision_slugs as $slug ) {
			$json_path = __DIR__ . '/../../blocks/account-detail/inner/' . $slug . '/block.json';
			$json      = json_decode( (string) file_get_contents( $json_path ), true );
			$this->assertSame(
				'dailyos/account-detail-' . $slug,
				$json['name'],
				$slug . ' must carry the account-detail- surface prefix to avoid 3-way name collision'
			);
		}
	}

	// ---- inner blocks: empty-state pattern ------------------------------

	/**
	 * 5 inner blocks (stakeholder-grid, recommended-actions,
	 * touchpoints-feed, open-loops-feed, unified-timeline) render the
	 * dailyos-empty-chip with a data-empty-reason when the envelope is
	 * absent — V1.1 §10 invariant "never silent-hidden".
	 *
	 * The 4 feed-shape blocks are surface-prefixed (dailyos/account-detail-*)
	 * to avoid the 3-way name collision with person-detail's top-level
	 * blocks of the same short slug, which would otherwise first-win at
	 * register_block_type and starve the inner block of context.
	 */
	public function test_complex_inner_blocks_render_empty_chip_on_absent_envelope(): void {
		$cases = [
			'stakeholder-grid'    => 'dailyos_stakeholder_grid_render',
			'recommended-actions' => 'dailyos_account_detail_recommended_actions_render',
			'touchpoints-feed'    => 'dailyos_account_detail_touchpoints_feed_render',
			'open-loops-feed'     => 'dailyos_account_detail_open_loops_feed_render',
			'unified-timeline'    => 'dailyos_account_detail_unified_timeline_render',
		];
		foreach ( $cases as $slug => $fn ) {
			include_once __DIR__ . '/../../blocks/account-detail/inner/' . $slug . '/render-functions.php';
			$this->assertTrue( function_exists( $fn ), $fn );
			$html = $fn( [], '', null );
			$this->assertStringContainsString(
				'dailyos-empty-chip',
				$html,
				$slug . ' must render the empty chip when envelope is absent'
			);
			$this->assertStringContainsString(
				'data-empty-reason=',
				$html,
				$slug . ' must carry data-empty-reason per §10 invariant'
			);
		}
	}

	// ---- envelope helpers: section state lookup -------------------------

	/**
	 * Verify `dailyos_envelope_section` returns present=true/item_count when the
	 * section is populated; falls back to reason on Empty variants.
	 */
	public function test_envelope_section_reads_present_and_empty_states(): void {
		$envelope = [
			'sections' => [
				'facts'      => [
					'kind'       => 'present',
					'item_count' => 4,
				],
				'open_loops' => [
					'kind'   => 'empty',
					'reason' => 'not_processed_yet',
				],
			],
		];
		$facts    = dailyos_envelope_section( $envelope, 'facts' );
		$this->assertTrue( $facts['present'] );
		$this->assertSame( 4, $facts['item_count'] );

		$loops = dailyos_envelope_section( $envelope, 'open_loops' );
		$this->assertFalse( $loops['present'] );
		$this->assertSame( 'not_processed_yet', $loops['reason'] );

		$missing = dailyos_envelope_section( $envelope, 'threads' );
		$this->assertFalse( $missing['present'] );
		$this->assertSame( 'not_available', $missing['reason'] );
	}

	// ---- envelope handle determinism (W1W2 L2 cycle-2 MEDIUM fix) -------

	/**
	 * Envelope handle is deterministic on envelope shape: two calls with
	 * the same envelope payload produce the same handle, and both hit the
	 * request-scoped cache so a single producer invocation serves outer +
	 * inner blocks. Regression guard for the
	 * spl_object_hash((object) $envelope) drift where every call cast to a
	 * fresh stdClass and emitted a new handle.
	 */
	public function test_envelope_handle_fallback_is_deterministic_on_shape(): void {
		// Strip envelopeRenderId so we exercise the fallback branch.
		$envelope = [
			'subject'  => [
				'kind'         => 'account',
				'id'           => 'acct-test-001',
				'displayLabel' => 'Generic Test Account',
			],
			'sections' => [
				'facts' => [
					'kind'       => 'present',
					'item_count' => 3,
				],
			],
		];

		$handle_a = dailyos_envelope_handle_from_response( $envelope, 'account', 'acct-test-001' );
		$handle_b = dailyos_envelope_handle_from_response( $envelope, 'account', 'acct-test-001' );

		$this->assertNotSame( '', $handle_a, 'handle emitted from fallback' );
		$this->assertSame(
			$handle_a,
			$handle_b,
			'identical envelope shape yields identical handle across calls (fallback determinism)'
		);

		// And the envelope is fetchable under that handle from the cache.
		$cached = dailyos_envelope_cache_get( $handle_a );
		$this->assertNotNull( $cached, 'envelope cached under deterministic handle' );
	}

	/**
	 * Production response shape: runtime returns
	 * `{ ok, request_id, ability: { ability_name, data, ... } }` per
	 * AbilityResponseJson::serialize in src-tauri/src/bridges/types.rs. The
	 * envelope-handle extractor MUST unwrap `$response['ability']['data']`,
	 * not fall through to the bare `$response` (which would cache the outer
	 * wrapper as if it were the envelope and starve every inner block of
	 * `sections`, triggering `not_available` chips).
	 */
	public function test_envelope_handle_unwraps_runtime_ability_data_shape(): void {
		$envelope         = [
			'envelopeRenderId' => 'env-acct-test-prod-001',
			'subject'          => [
				'kind' => 'account',
				'id'   => 'acct-test-prod',
			],
			'sections'         => [
				'facts' => [
					'kind'       => 'present',
					'item_count' => 1,
				],
			],
		];
		$runtime_response = [
			'ok'         => true,
			'request_id' => 'req-prod-001',
			'ability'    => [
				'ability_name'    => 'get_entity_intelligence',
				'ability_version' => 'v1.0.0',
				'schema_version'  => 1,
				'data'            => $envelope,
			],
		];

		$handle = dailyos_envelope_handle_from_response( $runtime_response, 'account', 'acct-test-prod' );
		$this->assertSame( 'env-acct-test-prod-001', $handle, 'envelopeRenderId extracted from ability.data path' );

		$cached = dailyos_envelope_cache_get( $handle );
		$this->assertIsArray( $cached, 'envelope cached under handle' );
		$this->assertArrayHasKey( 'sections', $cached, 'cached value is the envelope, not the runtime wrapper' );
		$this->assertSame( 'present', $cached['sections']['facts']['kind'] );
	}

	/**
	 * Entity-intelligence envelopes already carry rendered claim items; row
	 * rendering should use those before falling back to per-claim receipt fan-out.
	 */
	public function test_envelope_consume_claim_uses_embedded_rendered_claim_item(): void {
		$handle = dailyos_envelope_handle_from_response(
			[
				'ok'      => true,
				'ability' => [
					'ability_name' => 'get_entity_intelligence',
					'data'         => [
						'envelopeRenderId' => 'env-acct-test-embedded-001',
						'sections'         => [
							'facts' => [
								'kind'       => 'present',
								'item_count' => 1,
							],
						],
						'facts'            => [
							'items' => [
								[
									'claimId'      => 'claim-test-embedded-001',
									'renderedText' => [
										'text' => 'Embedded rendered claim',
									],
									'trustBand'    => 'likely_current',
								],
							],
						],
					],
				],
			],
			'account',
			'acct-test-embedded'
		);
		$this->assertSame( 'env-acct-test-embedded-001', $handle );

		$client = $this->fake_runtime_client_with_envelope(
			[
				'ok'      => true,
				'ability' => [
					'ability_name' => 'claim_receipt',
					'data'         => [
						'renderedText' => [
							'text' => 'Fallback rendered claim',
						],
					],
				],
			]
		);
		$this->register_runtime_client_filter( $client );

		$receipt = dailyos_envelope_consume_claim(
			[
				'claim_id'    => 'claim-test-embedded-001',
				'subject_ref' => [ 'account' => 'acct-test-embedded' ],
			],
			[]
		);

		$this->assertIsArray( $receipt );
		$this->assertSame( 'Embedded rendered claim', $receipt['renderedText']['text'] );
		$this->assertSame( 'likely_current', dailyos_receipt_trust_band( $receipt ) );
		$this->assertSame( 0, $client->calls, 'embedded envelope claim should avoid claim_receipt fan-out' );
	}

	/**
	 * Claim receipt fan-out returns the receipt DTO, not the runtime wrapper.
	 * The account-detail inner blocks consume this helper before row render.
	 */
	public function test_envelope_consume_claim_unwraps_claim_receipt_runtime_response(): void {
		$client = $this->fake_runtime_client_with_envelope(
			[
				'ok'      => true,
				'ability' => [
					'ability_name' => 'claim_receipt',
					'data'         => [
						'renderedText' => [
							'text' => 'Generic rendered claim',
						],
						'trust'        => [
							'band' => 'likely_current',
						],
					],
				],
			]
		);
		$this->register_runtime_client_filter( $client );

		$receipt = dailyos_envelope_consume_claim(
			[
				'claim_id'    => 'claim-test-001',
				'subject_ref' => [ 'account' => 'acct-test-001' ],
				'field_path'  => 'status',
			],
			[]
		);

		$this->assertIsArray( $receipt );
		$this->assertSame( 'Generic rendered claim', $receipt['renderedText']['text'] );
		$this->assertSame( 'claim_receipt', $client->requests[0]['ability'] );
		$this->assertSame( 1, $client->requests[0]['payload']['schemaVersion'] );
		$this->assertSame( 'claim', $client->requests[0]['payload']['target']['kind'] );
		$this->assertSame( 'entity_detail', $client->requests[0]['payload']['surface'] );
	}

	/**
	 * Hero rows use the new claim_receipt DTO for rendered text, trust, and
	 * provenance labels instead of falling back to opaque claim ids.
	 */
	public function test_account_hero_row_reads_claim_receipt_dto_fields(): void {
		$html = dailyos_account_hero_render_row(
			[ 'claim_id' => 'claim-test-001' ],
			[
				'renderedText' => [
					'text' => 'Generic rendered claim',
				],
				'trust'        => [
					'band' => 'likely_current',
				],
				'provenance'   => [
					'sources' => [
						[
							'label'    => 'Generic source',
							'redacted' => false,
						],
					],
				],
			]
		);

		$this->assertStringContainsString( 'Generic rendered claim', $html );
		$this->assertStringContainsString( 'data-trust-band="likely_current"', $html );
		$this->assertStringContainsString( 'Generic source', $html );
	}

	/**
	 * Round-trip: outer + inner block sharing an envelopeRenderId both
	 * resolve to the same cached envelope without re-invoking the producer.
	 */
	public function test_outer_and_inner_share_envelope_handle_single_fetch(): void {
		$response = $this->envelope_response_present();
		$client   = $this->fake_runtime_client_with_envelope( $response );
		$this->register_runtime_client_filter( $client );

		// Outer-equivalent: emit handle from a response.
		$outer_handle         = dailyos_envelope_handle_from_response( $response, 'account', 'acct-test-001' );
		$outer_calls_baseline = $client->calls;

		// Inner-equivalent: resolve the same handle. With the deterministic
		// handle + cache hit, dailyos_resolve_envelope short-circuits and
		// never invokes the runtime client.
		$resolved = dailyos_resolve_envelope( $outer_handle, 'account', 'acct-test-001', [] );

		$this->assertIsArray( $resolved, 'inner resolve returns cached envelope' );
		$this->assertSame(
			$outer_calls_baseline,
			$client->calls,
			'inner block hits the cache; producer is NOT re-invoked'
		);
	}

	// ---- filesystem pattern present -------------------------------------

	/**
	 * Default composition ships as a filesystem pattern per AC-462.8.
	 *
	 * Pattern file uses header-style registration (parsed by the plugin's
	 * `register_block_patterns()` loader at `class-dailyos-plugin.php:195`)
	 * matching project-detail-default / person-detail-default /
	 * meeting-detail-default sibling files. The previous direct
	 * `register_block_pattern()` direct-call style never registered because
	 * the loader only reads files with the standard `Title:` / `Slug:`
	 * docblock headers (V1.2 substrate-trim convert at `wave/v1.4.4-w1-stage1a`).
	 */
	public function test_filesystem_pattern_ships_default_composition(): void {
		$pattern = __DIR__ . '/../../patterns/account-detail-default.php';
		$this->assertFileExists( $pattern );
		$contents = file_get_contents( $pattern );
		// Header-style registration: Title + Slug + Block Types headers.
		$this->assertMatchesRegularExpression(
			'/^\s*\*\s*Slug:\s*dailyos\/account-detail-default\s*$/m',
			$contents,
			'pattern declares the dailyos/account-detail-default Slug header'
		);
		$this->assertMatchesRegularExpression(
			'/^\s*\*\s*Title:\s*\S+/m',
			$contents,
			'pattern declares a Title header'
		);
		$this->assertStringContainsString( '<!-- wp:dailyos/account-detail -->', $contents );
		// Mirrors the canonical 24-chapter ordering.
		$this->assertStringContainsString( '<!-- wp:dailyos/account-hero /-->', $contents );
		$this->assertStringContainsString( '<!-- wp:dailyos/finis-marker /-->', $contents );
	}

	// ---- helpers --------------------------------------------------------

	/**
	 * Build a representative envelope-response payload (present sections).
	 */
	private function envelope_response_present(): array {
		return [
			'ok'       => true,
			'envelope' => [
				'envelopeRenderId' => 'env-acct-test-001-v1',
				'subject'          => [
					'kind'         => 'account',
					'id'           => 'acct-test-001',
					'displayLabel' => 'Generic Test Account',
				],
				'sections'         => [
					'facts'              => [
						'kind'       => 'present',
						'item_count' => 3,
					],
					'health'             => [
						'kind'       => 'present',
						'item_count' => 2,
					],
					'metadata_proposals' => [
						'kind'   => 'empty',
						'reason' => 'no_evidence_backed_proposal',
					],
					'open_loops'         => [
						'kind'       => 'present',
						'item_count' => 1,
					],
					'touchpoints'        => [
						'kind'       => 'present',
						'item_count' => 4,
					],
					'threads'            => [
						'kind'   => 'empty',
						'reason' => 'not_processed_yet',
					],
					'record'             => [
						'kind'       => 'present',
						'item_count' => 6,
					],
				],
			],
		];
	}

	/**
	 * Build a fake runtime client whose invoke_ability captures the
	 * call shape and returns the queued response. Required for AC-W1.9.
	 *
	 * @param array<string,mixed> $response Response payload.
	 */
	private function fake_runtime_client_with_envelope( array $response ): object {
		return new class( $response ) {
			/**
			 * @var array
			 */
			public array $response;
			/**
			 * @var int
			 */
			public int $calls = 0;
			/**
			 * @var array<int,array<string,mixed>>
			 */
			public array $requests = [];
			public function __construct( array $response ) {
				$this->response = $response;
			}
			public function invoke_ability( string $ability, array $payload, array $scope_set ) {
				++$this->calls;
				$this->requests[] = [
					'ability'   => $ability,
					'payload'   => $payload,
					'scope_set' => $scope_set,
				];
				return $this->response;
			}
		};
	}

	/**
	 * Register the runtime-client filter so dailyos_account_detail_render
	 * picks up the fake.
	 */
	private function register_runtime_client_filter( object $client ): void {
		add_filter(
			'dailyos_runtime_client_for_block',
			static function () use ( $client ) {
				return $client;
			}
		);
	}
}
