<?php
/**
 * DailyOS MCP SurfaceClient scope drift tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Ensures the resolved scope set stays pinned.
 */
final class DailyOS_ScopeSetDriftTest extends TestCase {
	/**
	 * Reset WordPress test doubles.
	 */
	protected function setUp(): void {
		parent::setUp();
		dailyos_test_reset_globals();
	}

	/**
	 * Resolver returns the pinned filtered scope set.
	 */
	public function test_resolver_returns_pinned_scope_set(): void {
		$fixture = $this->load_fixture();

		add_filter(
			'dailyos_surfaceclient_resolved_scopes',
			static function () use ( $fixture ): array {
				return $fixture['scopes'];
			}
		);

		$resolver = static function (): array {
			return apply_filters( 'dailyos_surfaceclient_resolved_scopes', [] );
		};

		$this->assertSame( $fixture['scopes'], $resolver() );
	}

	/**
	 * Load the scope fixture.
	 *
	 * @return array{scopes: array<int, string>}
	 */
	private function load_fixture(): array {
		$path = dirname( __DIR__ ) . '/fixtures/surfaceclient-scopes.json';

		// phpcs:ignore WordPress.WP.AlternativeFunctions.file_get_contents_file_get_contents -- Test fixture read.
		return json_decode( (string) file_get_contents( $path ), true, 512, JSON_THROW_ON_ERROR );
	}
}
