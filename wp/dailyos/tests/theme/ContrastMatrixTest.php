<?php
/**
 * WCAG 2.x contrast matrix tests for W3 style variations.
 *
 * Per L0 Packet E §8.6: body-text foreground/background pairings declared
 * by each FSE style variation must clear WCAG AAA (≥7:1) and match the
 * spec's published ratios.
 *
 * Slugs are read from the variation JSON files; hex values are read from
 * `src/styles/design-tokens.css` so the test breaks loudly if either
 * surface drifts.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use PHPUnit\Framework\TestCase;

/**
 * Covers W3 contrast-matrix invariant for editorial-light and
 * high-contrast variations.
 */
final class DailyOS_ContrastMatrixTest extends TestCase {
	/**
	 * AAA contrast threshold for body text per WCAG 2.2 SC 1.4.6.
	 */
	private const AAA_THRESHOLD = 7.0;

	/**
	 * Allowable absolute deviation from the spec-published ratio.
	 *
	 * Generous enough to absorb floating-point rounding while still catching
	 * a real token drift (which would shift the ratio by ≥0.1).
	 */
	private const RATIO_TOLERANCE = 0.05;

	/**
	 * Spec-published body-text contrast ratios (L0 Packet E §5.5 / §8.6).
	 *
	 * @return array<string, array{slug: string, expected_ratio: float}>
	 */
	public static function variation_expectations(): array {
		return [
			'editorial-light' => [
				'slug'           => 'editorial-light',
				'expected_ratio' => 13.82,
			],
			'high-contrast'   => [
				'slug'           => 'high-contrast',
				'expected_ratio' => 14.55,
			],
		];
	}

	/**
	 * @return array<int, array{0: string, 1: float}>
	 */
	public static function variation_provider(): array {
		$rows = [];
		foreach ( self::variation_expectations() as $row ) {
			$rows[] = [ $row['slug'], $row['expected_ratio'] ];
		}
		return $rows;
	}

	/**
	 * Each variation's body-text pairing clears WCAG AAA and matches spec.
	 *
	 * @dataProvider variation_provider
	 */
	public function test_body_text_contrast_meets_aaa_and_matches_spec(
		string $slug,
		float $expected_ratio
	): void {
		$variation = self::load_variation( $slug );

		$background_slug = self::slug_from_token_reference(
			(string) ( $variation['styles']['color']['background'] ?? '' )
		);
		$text_slug       = self::slug_from_token_reference(
			(string) ( $variation['styles']['color']['text'] ?? '' )
		);

		$this->assertNotSame(
			'',
			$background_slug,
			sprintf( '%s.json must declare styles.color.background as a token reference.', $slug )
		);
		$this->assertNotSame(
			'',
			$text_slug,
			sprintf( '%s.json must declare styles.color.text as a token reference.', $slug )
		);

		$background_hex = self::resolve_token_hex( $background_slug );
		$text_hex       = self::resolve_token_hex( $text_slug );

		$ratio = self::wcag_contrast( $background_hex, $text_hex );

		$this->assertGreaterThanOrEqual(
			self::AAA_THRESHOLD,
			$ratio,
			sprintf(
				'%s body-text contrast must clear WCAG AAA (≥%.1f). %s × %s yielded %.4f.',
				$slug,
				self::AAA_THRESHOLD,
				$background_slug,
				$text_slug,
				$ratio
			)
		);

		$this->assertEqualsWithDelta(
			$expected_ratio,
			$ratio,
			self::RATIO_TOLERANCE,
			sprintf(
				'%s body-text contrast (%.4f) drifted from spec-published %.2f beyond tolerance %.2f. '
				. 'Either a palette token shifted hex value or §5.5/§8.6 needs re-publishing.',
				$slug,
				$ratio,
				$expected_ratio,
				self::RATIO_TOLERANCE
			)
		);
	}

	/**
	 * Decode a style variation JSON file.
	 *
	 * @return array<string, mixed>
	 */
	private static function load_variation( string $slug ): array {
		$path = dirname( __DIR__, 2 ) . '/theme/styles/' . $slug . '.json';
		$raw  = (string) file_get_contents( $path );

		$decoded = json_decode( $raw, true );

		return is_array( $decoded ) ? $decoded : [];
	}

	/**
	 * Extract the palette slug from a `var(--wp--preset--color--<slug>)` reference.
	 */
	private static function slug_from_token_reference( string $reference ): string {
		if ( 1 === preg_match( '/var\(--wp--preset--color--([a-z0-9-]+)\)/', $reference, $matches ) ) {
			return $matches[1];
		}

		return '';
	}

	/**
	 * Look up the hex value of a design-system color token by slug.
	 *
	 * Reads from `src/styles/design-tokens.css` (the Tauri-side authority
	 * mirrored into `theme.json` per ADR-0130 token-discipline rules).
	 */
	private static function resolve_token_hex( string $slug ): string {
		$repo_root = dirname( __DIR__, 4 );
		$tokens    = (string) file_get_contents( $repo_root . '/src/styles/design-tokens.css' );

		$pattern = sprintf( '/--color-%s:\s*(#[0-9a-fA-F]{3,8})\s*;/', preg_quote( $slug, '/' ) );

		if ( 1 !== preg_match( $pattern, $tokens, $matches ) ) {
			throw new RuntimeException(
				sprintf( 'Design token --color-%s not found in src/styles/design-tokens.css.', $slug )
			);
		}

		return $matches[1];
	}

	/**
	 * WCAG relative luminance per WCAG 2.x §1.4.3.
	 */
	private static function wcag_luminance( string $hex ): float {
		$hex = ltrim( $hex, '#' );

		if ( 3 === strlen( $hex ) ) {
			$hex = $hex[0] . $hex[0] . $hex[1] . $hex[1] . $hex[2] . $hex[2];
		}

		$r = hexdec( substr( $hex, 0, 2 ) ) / 255.0;
		$g = hexdec( substr( $hex, 2, 2 ) ) / 255.0;
		$b = hexdec( substr( $hex, 4, 2 ) ) / 255.0;

		$r = $r <= 0.03928 ? $r / 12.92 : ( ( $r + 0.055 ) / 1.055 ) ** 2.4;
		$g = $g <= 0.03928 ? $g / 12.92 : ( ( $g + 0.055 ) / 1.055 ) ** 2.4;
		$b = $b <= 0.03928 ? $b / 12.92 : ( ( $b + 0.055 ) / 1.055 ) ** 2.4;

		return 0.2126 * $r + 0.7152 * $g + 0.0722 * $b;
	}

	/**
	 * WCAG contrast ratio between two hex colors.
	 */
	private static function wcag_contrast( string $a, string $b ): float {
		$la = self::wcag_luminance( $a );
		$lb = self::wcag_luminance( $b );

		return ( max( $la, $lb ) + 0.05 ) / ( min( $la, $lb ) + 0.05 );
	}
}
