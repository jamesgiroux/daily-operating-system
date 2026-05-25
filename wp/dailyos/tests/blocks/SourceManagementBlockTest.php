<?php
/**
 * Source Management block tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

require_once __DIR__ . '/../../blocks/source-management/render-functions.php';

/**
 * Tests for the dailyos/source-management WP block render path.
 */
final class DailyOS_SourceManagementBlockTest extends TestCase {
	protected function setUp(): void {
		parent::setUp();
		if ( function_exists( 'dailyos_test_reset_globals' ) ) {
			dailyos_test_reset_globals();
		}
	}

	public function test_block_json_uses_dailyos_category_and_dynamic_render(): void {
		$block_json = json_decode(
			(string) file_get_contents( __DIR__ . '/../../blocks/source-management/block.json' ),
			true
		);

		$this->assertSame( 'dailyos/source-management', $block_json['name'] );
			$this->assertSame( 'dailyos', $block_json['category'] );
			$this->assertSame( 'file:./render.php', $block_json['render'] );
			$this->assertSame( 'file:./view.js', $block_json['viewScript'] );
		$this->assertArrayHasKey( 'entity_type', $block_json['attributes'] );
		$this->assertArrayHasKey( 'entity_id', $block_json['attributes'] );
		$this->assertArrayNotHasKey( 'sources', $block_json['attributes'] );
		$this->assertArrayNotHasKey( 'source_handle', $block_json['attributes'] );
	}

	public function test_default_render_is_unavailable_without_runtime_projection(): void {
		$html = dailyos_source_management_render(
			[
				'entity_type' => 'account',
				'entity_id'   => 'acct-test-001',
			]
		);

		$this->assertStringContainsString( 'data-dailyos-surface="source-management"', $html );
		$this->assertStringContainsString( 'data-dailyos-state="unavailable"', $html );
		$this->assertStringContainsString( 'Sources are temporarily unavailable.', $html );
		$this->assertStringNotContainsString( 'acct-test-001', $html );
	}

	public function test_payload_render_redacts_raw_source_values(): void {
		$html = dailyos_source_management_render_payload(
			[
				'actionPolicy' => [
					'disabledReason' => 'write_actions_deferred',
				],
				'sources'      => [
					[
							'sourceKey'        => 'source:v1:abcdefghijklmnopqrstuvwxyzABCDEF0123456789_-',
						'sourceHandle'     => 'source_opaque_123',
						'file_id'          => 'pathhash-alpha',
						'link_id'          => 'link-alpha',
						'run_id'           => 'run-alpha',
						'canonical_path'   => '/Users/example/workspace/private.md',
						'claim_text'       => 'raw claim text must not render',
						'sourceKind'       => 'mcp_placement',
						'lifecycleState'   => 'quarantined',
						'category'         => 'notes',
							'sourceAsof'       => '2026-05-24T10:00:00Z',
							'entity'           => [
								'entityType' => 'account',
								'entityId'   => 'acct-test-001',
							],
							'latestRun'        => [
								'status'             => 'success',
								'claimCountProduced' => 2,
							],
							'ingestionRuns'    => [
								[
									'status'             => 'success',
									'claimCountProduced' => 2,
								],
								[
									'status'             => 'failed',
									'claimCountProduced' => 0,
								],
							],
							'trustBandSummary' => [
							'total'             => 3,
							'likelyCurrent'     => 1,
							'useWithCaution'    => 1,
							'needsVerification' => 1,
							'unscored'          => 0,
						],
							'actions'          => [
								'canReingest'    => true,
								'canQuarantine'  => true,
								'canRelink'      => true,
								'disabledReason' => '',
							],
					],
				],
			]
		);

		$this->assertStringContainsString( 'Workspace document', $html );
		$this->assertStringContainsString( 'Needs review', $html );
		$this->assertStringContainsString( 'notes', $html );
			$this->assertStringContainsString( 'May 24, 2026', $html );
			$this->assertStringContainsString( 'Success, 2 claims', $html );
			$this->assertStringContainsString( 'Ingestion run history', $html );
			$this->assertStringContainsString( 'Failed, 0 claims', $html );
			$this->assertStringContainsString( '1 likely current', $html );
			$this->assertStringContainsString( 'Re-ingest', $html );
			$this->assertStringContainsString( 'Quarantine', $html );
			$this->assertStringContainsString( 'Re-link', $html );
			$this->assertStringContainsString( 'data-dailyos-source-action="reingest"', $html );
			$this->assertStringContainsString( 'data-dailyos-source-key="source:v1:abcdefghijklmnopqrstuvwxyzABCDEF0123456789_-"', $html );
			$this->assertStringNotContainsString( 'source_opaque_123', $html );
		$this->assertStringNotContainsString( 'pathhash-alpha', $html );
		$this->assertStringNotContainsString( 'link-alpha', $html );
		$this->assertStringNotContainsString( 'run-alpha', $html );
		$this->assertStringNotContainsString( '/Users/example/workspace/private.md', $html );
		$this->assertStringNotContainsString( 'raw claim text must not render', $html );
	}
}
