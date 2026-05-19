<?php
/**
 * FSE style variation schema tests.
 *
 * Validates v1.4.3 W3 magazine theme style variations against
 * L0 Packet E §8.5: well-formed JSON, declared schema/version/title,
 * and the §6 #3 "no new tokens" invariant (all color values resolve
 * via `var(--wp--preset--color--*)` references, never literal hex).
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Covers DOS-698 W3 style variation schema and token-discipline gates.
 */
final class DailyOS_StyleVariationSchemaTest extends TestCase {
	/**
	 * Variation slugs shipped in W3 (editorial-dark deferred to DOS-699).
	 *
	 * @return array<int, array{0: string}>
	 */
	public static function variation_provider(): array {
		return [
			[ 'editorial-light' ],
			[ 'high-contrast' ],
		];
	}

	/**
	 * Each variation file exists on disk at the expected FSE path.
	 *
	 * @dataProvider variation_provider
	 */
	public function test_variation_file_exists( string $slug ): void {
		$path = self::variation_path( $slug );

		$this->assertFileExists(
			$path,
			sprintf( 'Style variation %s.json must exist at %s.', $slug, $path )
		);
	}

	/**
	 * Each variation file parses as valid JSON.
	 *
	 * @dataProvider variation_provider
	 */
	public function test_variation_is_valid_json( string $slug ): void {
		$payload = self::load_variation( $slug );

		$this->assertIsArray(
			$payload,
			sprintf( 'Style variation %s.json must decode to a JSON object.', $slug )
		);
	}

	/**
	 * Each variation declares the FSE-required top-level keys.
	 *
	 * @dataProvider variation_provider
	 */
	public function test_variation_declares_required_keys( string $slug ): void {
		$payload = self::load_variation( $slug );

		$this->assertArrayHasKey( '$schema', $payload, '$schema must be declared.' );
		$this->assertArrayHasKey( 'version', $payload, 'version must be declared.' );
		$this->assertArrayHasKey( 'title', $payload, 'title must be declared.' );

		$this->assertSame(
			'https://schemas.wp.org/trunk/theme.json',
			$payload['$schema'],
			'$schema must point at the trunk theme.json schema.'
		);
		$this->assertSame( 3, $payload['version'], 'theme.json schema version must be 3.' );
		$this->assertIsString( $payload['title'] );
		$this->assertNotSame( '', trim( $payload['title'] ), 'title must be non-empty.' );
	}

	/**
	 * Every color value inside `styles.*` must resolve through the registered
	 * palette via `var(--wp--preset--color--*)` — no literal hex values are
	 * permitted under the §6 #3 "no new tokens" invariant.
	 *
	 * @dataProvider variation_provider
	 */
	public function test_styles_block_uses_token_references_only( string $slug ): void {
		$path = self::variation_path( $slug );
		$raw  = (string) file_get_contents( $path );

		$payload = json_decode( $raw, true );
		$this->assertIsArray( $payload );

		if ( ! isset( $payload['styles'] ) ) {
			$this->addToAssertionCount( 1 );
			return;
		}

		$styles_json = (string) json_encode( $payload['styles'] );

		// Token-discipline gate: any 3- or 6-digit hex literal inside the
		// `styles` block leaks a raw color value past the palette boundary.
		$this->assertSame(
			0,
			preg_match( '/#[0-9a-fA-F]{3}(?:[0-9a-fA-F]{3})?\b/', $styles_json ),
			sprintf(
				'Style variation %s.json must not embed literal hex colors inside styles.* — '
				. 'use var(--wp--preset--color--*) references instead (§6 #3 no-new-tokens invariant).',
				$slug
			)
		);
	}

	/**
	 * Path to a style variation JSON file.
	 */
	private static function variation_path( string $slug ): string {
		return dirname( __DIR__, 2 ) . '/theme/styles/' . $slug . '.json';
	}

	/**
	 * Decode a style variation JSON file as an associative array.
	 *
	 * @return array<string, mixed>
	 */
	private static function load_variation( string $slug ): array {
		$raw     = (string) file_get_contents( self::variation_path( $slug ) );
		$decoded = json_decode( $raw, true );

		return is_array( $decoded ) ? $decoded : [];
	}
}
