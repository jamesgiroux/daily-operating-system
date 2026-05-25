<?php
/**
 * Composition diagnostic render regression tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

require_once dirname( __DIR__ ) . '/bootstrap.php';
require_once dirname( __DIR__, 2 ) . '/blocks/account-overview/render-functions.php';
require_once dirname( __DIR__, 2 ) . '/blocks/avatar/render-functions.php';
require_once dirname( __DIR__, 2 ) . '/blocks/entity-chip/render-functions.php';
require_once dirname( __DIR__, 2 ) . '/blocks/freshness-indicator/render-functions.php';
require_once dirname( __DIR__, 2 ) . '/blocks/health-badge/render-functions.php';
require_once dirname( __DIR__, 2 ) . '/blocks/intelligence-quality-badge/render-functions.php';
require_once dirname( __DIR__, 2 ) . '/blocks/provenance-tag/render-functions.php';
require_once dirname( __DIR__, 2 ) . '/blocks/score-band/render-functions.php';
require_once dirname( __DIR__, 2 ) . '/blocks/status-dot/render-functions.php';
require_once dirname( __DIR__, 2 ) . '/blocks/trust-band-badge/render-functions.php';
require_once dirname( __DIR__, 2 ) . '/blocks/type-badge/render-functions.php';

/**
 * Covers the DOS-737 diagnostic split for composition-backed blocks.
 */
final class DailyOS_CompositionDiagnosticRenderTest extends TestCase {
	protected function setUp(): void {
		parent::setUp();
		dailyos_test_reset_globals();
	}

	/**
	 * @return array<string, array{0: callable(array<string, mixed>): string}>
	 */
	public static function empty_shell_block_provider(): array {
		return [
			'account-overview' => [ 'dailyos_account_overview_render' ],
			'entity-chip'      => [ 'dailyos_entity_chip_render' ],
			'provenance-tag'   => [ 'dailyos_provenance_tag_render' ],
			'score-band'       => [ 'dailyos_score_band_render' ],
			'status-dot'       => [ 'dailyos_status_dot_render' ],
			'type-badge'       => [ 'dailyos_type_badge_render' ],
		];
	}

	/**
	 * @return array<string, array{0: callable(array<string, mixed>): string}>
	 */
	public static function composition_block_provider(): array {
		return [
			'account-overview'           => [ 'dailyos_account_overview_render' ],
			'avatar'                     => [ 'dailyos_avatar_render' ],
			'entity-chip'                => [ 'dailyos_entity_chip_render' ],
			'freshness-indicator'        => [ 'dailyos_freshness_indicator_render' ],
			'health-badge'               => [ 'dailyos_health_badge_render' ],
			'intelligence-quality-badge' => [ 'dailyos_intelligence_quality_badge_render' ],
			'provenance-tag'             => [ 'dailyos_provenance_tag_render' ],
			'score-band'                 => [ 'dailyos_score_band_render' ],
			'status-dot'                 => [ 'dailyos_status_dot_render' ],
			'trust-band-badge'           => [ 'dailyos_trust_band_badge_render' ],
			'type-badge'                 => [ 'dailyos_type_badge_render' ],
		];
	}

	/**
	 * Missing composition remains the normal unconfigured empty path, but carries a diagnostic reason.
	 *
	 * @param callable(array<string, mixed>): string $render Render function.
	 */
	#[\PHPUnit\Framework\Attributes\DataProvider( 'empty_shell_block_provider' )]
	public function test_missing_composition_id_renders_empty_shell_diagnostic( callable $render ): void {
		add_filter(
			'dailyos_runtime_client_for_block',
			function (): void {
				$this->fail( 'Missing composition_id must not resolve the runtime client.' );
			},
			10,
			0
		);

		$html = $render( [] );

		$this->assertStringContainsString( 'is-empty', $html );
		$this->assertStringContainsString( 'data-empty-reason="missing_composition_id"', $html );
		$this->assertStringNotContainsString( 'runtime_unavailable', $html );
		$this->assertStringNotContainsString( 'Runtime unavailable', $html );
	}

	/**
	 * A configured composition with no runtime client is runtime-unavailable, not missing-composition.
	 *
	 * @param callable(array<string, mixed>): string $render Render function.
	 */
	#[\PHPUnit\Framework\Attributes\DataProvider( 'composition_block_provider' )]
	public function test_configured_composition_without_runtime_client_renders_runtime_unavailable( callable $render ): void {
		$html = $render(
			[
				'composition_id'      => 'composition-test-001',
				'composition_version' => 1,
			]
		);

		$this->assertStringNotContainsString( 'missing_composition_id', $html );
		$this->assertMatchesRegularExpression( '/runtime[_-]unavailable|Runtime unavailable/i', $html );
	}
}
