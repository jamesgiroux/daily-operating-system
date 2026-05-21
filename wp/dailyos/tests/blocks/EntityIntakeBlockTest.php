<?php
/**
 * Entity Intake block tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

require_once __DIR__ . '/../../blocks/entity-intake/render-functions.php';

final class DailyOS_EntityIntakeBlockTest extends TestCase {
	protected function setUp(): void {
		parent::setUp();
		if ( function_exists( 'dailyos_test_reset_globals' ) ) {
			dailyos_test_reset_globals();
		}
	}

	public function test_block_json_uses_dailyos_category_and_dynamic_render(): void {
		$block_json = json_decode(
			(string) file_get_contents( __DIR__ . '/../../blocks/entity-intake/block.json' ),
			true
		);

		$this->assertSame( 'dailyos/entity-intake', $block_json['name'] );
		$this->assertSame( 'dailyos', $block_json['category'] );
		$this->assertSame( 'file:./render.php', $block_json['render'] );
	}

	public function test_renders_fixture_claim_display_text_with_trust_band(): void {
		add_filter(
			'dailyos_runtime_client_for_block',
			static fn () => new class() {
				public function invoke_ability( string $name, array $payload, array $scopes ): array {
					TestCase::assertSame( 'entity_intake_render', $name );
					TestCase::assertSame( [ 'read.entity_intelligence' ], $scopes );
					TestCase::assertSame( 'account', $payload['entityType'] );

					return [
						'data' => [
							'claims' => [
								[
									'claimId'     => 'claim-1',
									'displayText' => 'Acme budget owner approved the workspace plan',
									'trustBand'   => 'likely_current',
									'sensitivity' => 'public',
								],
							],
						],
					];
				}
			},
			5,
			1
		);

		$html = dailyos_entity_intake_render(
			[
				'entity_type' => 'account',
				'entity_id'   => 'acct_acme',
				'file_ref'    => 'accounts/acme/brief.md',
			]
		);

		$this->assertStringContainsString( 'Acme budget owner approved the workspace plan', $html );
		$this->assertStringContainsString( 'data-band="likely_current"', $html );
		$this->assertStringContainsString( 'data-claim-id="claim-1"', $html );
	}

	public function test_path_traversal_renders_error_state(): void {
		$html = dailyos_entity_intake_render(
			[
				'entity_type' => 'account',
				'entity_id'   => 'acct_acme',
				'file_ref'    => '../outside.md',
			]
		);

		$this->assertStringContainsString( 'data-error-code="PathTraversalAttempt"', $html );
	}
}
