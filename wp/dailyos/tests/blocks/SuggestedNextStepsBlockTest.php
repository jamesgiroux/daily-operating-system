<?php
/**
 * Suggested Next Steps block tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

require_once __DIR__ . '/../../blocks/suggested-next-steps/render-functions.php';

/**
 * @covers dailyos_suggested_next_steps_render
 * @covers dailyos_suggested_next_steps_extract_items
 */
final class DailyOS_SuggestedNextStepsBlockTest extends TestCase {
	/**
	 * Reset WP shim globals.
	 */
	protected function setUp(): void {
		parent::setUp();
		if ( function_exists( 'dailyos_test_reset_globals' ) ) {
			dailyos_test_reset_globals();
		}
		unset( $GLOBALS['dailyos_test_filters']['dailyos_runtime_client_for_block'] );
		unset( $GLOBALS['dailyos_test_filters']['dailyos_surfaceclient_resolved_scopes'] );
		unset( $GLOBALS['dailyos_test_filters']['dailyos_suggested_next_steps_feedback_enabled'] );
	}

	/**
	 * Block.json declares the W3-A metadata contract.
	 */
	public function test_block_json_declares_contract(): void {
		$block_json = json_decode(
			(string) file_get_contents( __DIR__ . '/../../blocks/suggested-next-steps/block.json' ),
			true
		);

		$this->assertIsArray( $block_json );
		$this->assertSame( 'dailyos/suggested-next-steps', $block_json['name'] );
		$this->assertSame( 3, $block_json['apiVersion'] );
		$this->assertSame( [ 'dailyos/entityType', 'dailyos/entityId' ], $block_json['usesContext'] );
		$this->assertArrayNotHasKey( 'parent', $block_json );
		$this->assertFalse( $block_json['supports']['html'] );
		$this->assertFalse( $block_json['supports']['reusable'] );
		$this->assertFalse( $block_json['supports']['inserter'] );
		$this->assertSame( 5, $block_json['attributes']['maxItems']['default'] );
		$this->assertSame( 'string', $block_json['attributes']['headingLabel']['type'] );
		$this->assertSame( 'file:./render.php', $block_json['render'] );
		$this->assertSame( 'file:./style.css', $block_json['style'] );
		$this->assertSame( 'file:./view.js', $block_json['viewScript'] );
	}

	/**
	 * Parser accepts all runtime response shapes named in the plan.
	 */
	public function test_extract_items_accepts_supported_response_shapes(): void {
		$items = [ $this->item( 'rec-claim-1' ) ];

		$this->assertSame(
			$items,
			dailyos_suggested_next_steps_extract_items( [ 'ability' => [ 'data' => [ 'items' => $items ] ] ] )
		);
		$this->assertSame(
			$items,
			dailyos_suggested_next_steps_extract_items( [ 'data' => [ 'items' => $items ] ] )
		);
		$this->assertSame(
			$items,
			dailyos_suggested_next_steps_extract_items( [ 'items' => $items ] )
		);
		$this->assertNull( dailyos_suggested_next_steps_extract_items( [ 'data' => [] ] ) );
	}

	/**
	 * Render invokes the list ability with the camelCase W3-A payload.
	 */
	public function test_render_invokes_list_ability_with_subject_surface_and_capped_max_items(): void {
		$client = $this->fake_runtime_client(
			[
				'ok'           => true,
				'subjectLabel' => 'Acme Corp',
				'items'        => [ $this->item( 'rec-claim-1' ) ],
			]
		);
		$this->register_runtime_client_filter( $client );
		$this->register_scope_filter( [ 'read.recommendations' ] );

		$html = dailyos_suggested_next_steps_render(
			[ 'maxItems' => 99 ],
			$this->block_context( 'account', 'acct-test-001' )
		);

		$this->assertSame( 1, $client->calls );
		$this->assertSame( 'list_suggested_next_steps', $client->requests[0]['ability'] );
		$this->assertSame( 1, $client->requests[0]['payload']['schemaVersion'] );
		$this->assertSame( [ 'account' => 'acct-test-001' ], $client->requests[0]['payload']['subject'] );
		$this->assertSame( 'entity_detail', $client->requests[0]['payload']['surface'] );
		$this->assertSame( 8, $client->requests[0]['payload']['maxItems'] );
		$this->assertArrayNotHasKey( 'schema_version', $client->requests[0]['payload'] );
		$this->assertArrayNotHasKey( 'max_items', $client->requests[0]['payload'] );
		$this->assertSame( [ 'read.recommendations' ], $client->requests[0]['scope_set'] );
		$this->assertStringContainsString( 'What&#039;s next with Acme Corp', $html );
	}

	/**
	 * Missing or unsupported context renders a visible empty chip.
	 */
	public function test_missing_subject_context_renders_empty_chip(): void {
		$html = dailyos_suggested_next_steps_render( [], $this->block_context( 'workspace', 'global' ) );

		$this->assertStringContainsString( 'dailyos-empty-chip', $html );
		$this->assertStringContainsString( 'data-empty-reason="missing_subject_context"', $html );
	}

	/**
	 * Surface wrappers and default headings match the reference surfaces.
	 *
	 * @dataProvider surface_provider
	 */
	public function test_surface_wrappers_and_default_headings(
		string $entity_type,
		string $entity_id,
		string $label,
		string $wrapper_class,
		string $heading
	): void {
		$client = $this->fake_runtime_client(
			[
				'ok'           => true,
				'subjectLabel' => $label,
				'items'        => [ $this->item( 'rec-' . $entity_type ) ],
			]
		);
		$this->register_runtime_client_filter( $client );

		$html = dailyos_suggested_next_steps_render( [], $this->block_context( $entity_type, $entity_id ) );

		$this->assertStringContainsString( $wrapper_class, $html );
		$this->assertStringContainsString( $heading, $html );
		$this->assertStringContainsString( 'data-surface="' . $entity_type . '"', $html );
		$this->assertStringContainsString( 'SuggestedNextSteps_section', $html );
	}

	/**
	 * @return array<string, array{0: string, 1: string, 2: string, 3: string, 4: string}>
	 */
	public static function surface_provider(): array {
		return [
			'account' => [ 'account', 'acct-1', 'Acme Corp', 'entity-detail_marginLabelSection', 'What&#039;s next with Acme Corp' ],
			'project' => [ 'project', 'proj-1', 'Platform Unification', 'entity-detail_chapterSection', 'What&#039;s next on Platform Unification' ],
			'person'  => [ 'person', 'person-1', 'Jen Park', 'entity-detail_chapterSectionWithPadding', 'Open threads with Jen Park' ],
			'meeting' => [ 'meeting', 'meeting-1', 'Renewal Review', 'meeting-intel_chapterSection', 'What to cover' ],
		];
	}

	/**
	 * HeadingLabel overrides the surface default.
	 */
	public function test_heading_label_override_wins(): void {
		$client = $this->fake_runtime_client(
			[
				'ok'    => true,
				'items' => [ $this->item( 'rec-claim-1' ) ],
			]
		);
		$this->register_runtime_client_filter( $client );

		$html = dailyos_suggested_next_steps_render(
			[ 'headingLabel' => 'Next best moves' ],
			$this->block_context( 'project', 'proj-1' )
		);

		$this->assertStringContainsString( 'Next best moves', $html );
		$this->assertStringNotContainsString( "What's next on", $html );
	}

	/**
	 * Long subject labels are truncated before heading interpolation.
	 */
	public function test_default_heading_truncates_long_subject_label(): void {
		$client = $this->fake_runtime_client(
			[
				'ok'           => true,
				'subjectLabel' => 'Very Long Customer Account Name That Should Stop Before Overflow',
				'items'        => [ $this->item( 'rec-claim-1' ) ],
			]
		);
		$this->register_runtime_client_filter( $client );

		$html = dailyos_suggested_next_steps_render( [], $this->block_context( 'account', 'acct-1' ) );

		$this->assertStringContainsString( 'Very Long Customer Account Name That...', $html );
		$this->assertStringNotContainsString( 'Should Stop Before Overflow', $html );
	}

	/**
	 * Only needs_verification gets a row-level modifier.
	 */
	public function test_trust_band_row_modifier_only_for_needs_verification(): void {
		$client = $this->fake_runtime_client(
			[
				'ok'    => true,
				'items' => [
					$this->item( 'rec-likely', 'likely_current' ),
					$this->item( 'rec-caution', 'use_with_caution' ),
					$this->item( 'rec-verify', 'needs_verification' ),
				],
			]
		);
		$this->register_runtime_client_filter( $client );

		$html = dailyos_suggested_next_steps_render( [], $this->block_context( 'person', 'person-1', 'Jen Park' ) );

		$this->assertSame( 1, substr_count( $html, 'suggested-next-steps_row--needsVerification' ) );
		$this->assertStringContainsString( 'TrustBandIndicator_likelyCurrent', $html );
		$this->assertStringContainsString( 'TrustBandIndicator_useWithCaution', $html );
		$this->assertStringContainsString( 'TrustBandIndicator_needsVerification', $html );
	}

	/**
	 * Feedback is disabled by default until the W4-A submit path lands.
	 */
	public function test_disabled_feedback_state_renders_banner_and_non_focusable_affordances(): void {
		$client = $this->fake_runtime_client(
			[
				'ok'    => true,
				'items' => [ $this->item( 'rec-claim-1' ) ],
			]
		);
		$this->register_runtime_client_filter( $client );

		$html = dailyos_suggested_next_steps_render( [], $this->block_context( 'meeting', 'meeting-1' ) );

		$this->assertStringContainsString( 'dailyos-info-chip', $html );
		$this->assertStringContainsString( 'aria-live="polite"', $html );
		$this->assertStringContainsString( 'Feedback opens on the next sync.', $html );
		$this->assertSame( 6, substr_count( $html, 'disabled-affordance' ) );
		$this->assertSame( 6, substr_count( $html, 'tabindex="-1"' ) );
		$this->assertSame( 6, substr_count( $html, 'aria-disabled="true"' ) );
	}

	/**
	 * Enabled affordances keep the reference ARIA disclosure contract.
	 */
	public function test_enabled_affordances_render_aria_labels_and_disclosure_controls(): void {
		$client = $this->fake_runtime_client(
			[
				'ok'    => true,
				'items' => [ $this->item( 'rec-claim-1' ) ],
			]
		);
		$this->register_runtime_client_filter( $client );
		$this->enable_feedback();

		$html = dailyos_suggested_next_steps_render( [], $this->block_context( 'account', 'acct-1', 'Acme Corp' ) );

		$this->assertStringNotContainsString( 'Feedback opens on the next sync.', $html );
		$this->assertStringNotContainsString( 'disabled-affordance', $html );
		$this->assertStringContainsString( 'aria-label="Convert this recommendation to an action"', $html );
		$this->assertStringContainsString( 'aria-label="Dismiss this recommendation"', $html );
		$this->assertStringContainsString( 'aria-label="More feedback options"', $html );
		$this->assertStringContainsString( 'aria-expanded="false"', $html );
		$this->assertStringContainsString( 'aria-controls="rec-claim-1-more"', $html );
		$this->assertStringContainsString( 'id="rec-claim-1-more"', $html );
		$this->assertStringContainsString( 'aria-hidden="true"', $html );
		$this->assertStringContainsString( 'data-feedback-kind="notUseful"', $html );
		$this->assertStringContainsString( 'data-feedback-kind="tooNoisy"', $html );
		$this->assertStringContainsString( 'data-feedback-kind="dismissWithReason"', $html );
	}

	/**
	 * Runtime and empty-list states render visible empty chips.
	 */
	public function test_runtime_errors_and_empty_items_render_visible_empty_chips(): void {
		$client = $this->fake_runtime_client( new WP_Error( 'runtime_failed', 'Runtime failed.' ) );
		$this->register_runtime_client_filter( $client );

		$html = dailyos_suggested_next_steps_render( [], $this->block_context( 'account', 'acct-1' ) );
		$this->assertStringContainsString( 'dailyos-empty-chip', $html );
		$this->assertStringContainsString( 'data-empty-reason="envelope_error"', $html );

		$this->setUp();
		$client = $this->fake_runtime_client( [ 'ok' => false ] );
		$this->register_runtime_client_filter( $client );
		$html = dailyos_suggested_next_steps_render( [], $this->block_context( 'project', 'proj-1' ) );
		$this->assertStringContainsString( 'data-empty-reason="envelope_error"', $html );

		$this->setUp();
		$client = $this->fake_runtime_client(
			[
				'ok'    => true,
				'items' => [],
			]
		);
		$this->register_runtime_client_filter( $client );
		$html = dailyos_suggested_next_steps_render( [], $this->block_context( 'person', 'person-1' ) );
		$this->assertStringContainsString( 'data-empty-reason="no_recommendations"', $html );
		$this->assertStringContainsString( 'No open threads.', $html );
	}

	/**
	 * Build a fake runtime client.
	 *
	 * @param mixed $response Response payload.
	 * @return object
	 */
	private function fake_runtime_client( $response ): object {
		return new class( $response ) {
			/**
			 * @var mixed
			 */
			public $response;
			/**
			 * @var int
			 */
			public int $calls = 0;
			/**
			 * @var array<int, array<string, mixed>>
			 */
			public array $requests = [];
			/**
			 * @param mixed $response Response payload.
			 */
			public function __construct( $response ) {
				$this->response = $response;
			}
			/**
			 * @param string               $ability   Ability name.
			 * @param array<string, mixed> $payload   Payload.
			 * @param array<int, string>   $scope_set Scope set.
			 * @return mixed
			 */
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
	 * Register the runtime-client filter.
	 */
	private function register_runtime_client_filter( object $client ): void {
		add_filter(
			'dailyos_runtime_client_for_block',
			static function () use ( $client ) {
				return $client;
			},
			10,
			1
		);
	}

	/**
	 * Register a scope-set filter.
	 *
	 * @param array<int, string> $scopes Scopes.
	 */
	private function register_scope_filter( array $scopes ): void {
		add_filter(
			'dailyos_surfaceclient_resolved_scopes',
			static function () use ( $scopes ) {
				return $scopes;
			},
			10,
			1
		);
	}

	/**
	 * Enable the feedback filter for affordance tests.
	 */
	private function enable_feedback(): void {
		add_filter(
			'dailyos_suggested_next_steps_feedback_enabled',
			static function () {
				return true;
			},
			10,
			3
		);
	}

	/**
	 * Build a WP_Block-like context object.
	 *
	 * @param string $entity_type Entity type.
	 * @param string $entity_id   Entity ID.
	 * @param string $label       Optional label.
	 * @return object
	 */
	private function block_context( string $entity_type, string $entity_id, string $label = '' ): object {
		$context = [
			'dailyos/entityType' => $entity_type,
			'dailyos/entityId'   => $entity_id,
		];
		if ( '' !== $label ) {
			$context['dailyos/entityLabel'] = $label;
		}

		return new class( $context ) {
			/**
			 * @var array<string, mixed>
			 */
			public array $context;
			/**
			 * @param array<string, mixed> $context Context.
			 */
			public function __construct( array $context ) {
				$this->context = $context;
			}
		};
	}

	/**
	 * Build a minimal suggested-next-step item.
	 *
	 * @param string $claim_id Claim ID.
	 * @param string $band     Trust band.
	 * @return array<string, mixed>
	 */
	private function item( string $claim_id, string $band = 'likely_current' ): array {
		return [
			'claimId'               => $claim_id,
			'headline'              => 'Send the H1 expansion follow-up',
			'whyThisNowSurfaceText' => 'Open loop since the last review.',
			'factorBand'            => 'openLoopRelated',
			'recommendedAction'     => [
				'kind'        => 'sendMessage',
				'entityLabel' => 'Jen Park',
				'channel'     => 'email',
			],
			'trustBand'             => $band,
			'receipt'               => [
				'trust'      => [
					'band' => $band,
				],
				'provenance' => [
					'sources' => [],
				],
			],
			'feedbackState'         => 'pending',
			'conversionState'       => [
				'kind' => 'notConverted',
			],
		];
	}
}
