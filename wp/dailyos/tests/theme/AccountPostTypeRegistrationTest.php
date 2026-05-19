<?php
/**
 * W3 magazine theme — dailyos_account CPT registration tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Plugin;
use PHPUnit\Framework\TestCase;

if ( ! function_exists( 'register_post_type' ) ) {
	/**
	 * Minimal stub mirroring core's `register_post_type()` so unit tests
	 * can observe the registration without a live WordPress.
	 *
	 * @param string               $post_type Post type slug.
	 * @param array<string, mixed> $args      Registration arguments.
	 */
	function register_post_type( string $post_type, array $args = [] ): object {
		$registered = (object) array_merge(
			[
				'name'         => $post_type,
				'public'       => false,
				'has_archive'  => false,
				'rewrite'      => false,
				'show_in_rest' => false,
				'rest_base'    => '',
				'supports'     => [],
			],
			$args
		);

		$GLOBALS['dailyos_test_registered_post_types'][ $post_type ] = $registered;

		return $registered;
	}
}

if ( ! function_exists( 'get_post_types' ) ) {
	/**
	 * Minimal stub mirroring core's `get_post_types()`.
	 *
	 * @param array<string, mixed> $args     Lookup filter (only `name` is honored).
	 * @param string               $output   Output format (ignored).
	 * @param string               $operator Logical operator (ignored).
	 * @return array<string, mixed>
	 */
	function get_post_types( array $args = [], string $output = 'names', string $operator = 'and' ): array {
		unset( $output, $operator );

		$registry = $GLOBALS['dailyos_test_registered_post_types'] ?? [];

		if ( isset( $args['name'] ) ) {
			$name = (string) $args['name'];
			return isset( $registry[ $name ] ) ? [ $name => $registry[ $name ] ] : [];
		}

		return $registry;
	}
}

/**
 * Asserts that the W3 magazine theme's `dailyos_account` CPT is
 * registered with the shape templates and rewrite rules depend on.
 *
 * Spec: L0 Packet E V1.4 §5.0 + §8.2.
 */
final class DailyOS_AccountPostTypeRegistrationTest extends TestCase {
	/**
	 * Reset registries before every test.
	 */
	protected function setUp(): void {
		parent::setUp();

		dailyos_test_reset_globals();
		$GLOBALS['dailyos_test_registered_post_types'] = [];
	}

	/**
	 * Calling `register_post_types()` records the dailyos_account CPT.
	 */
	public function test_register_post_types_records_dailyos_account(): void {
		DailyOS_Plugin::instance()->register_post_types();

		$post_types = get_post_types( [ 'name' => 'dailyos_account' ] );

		$this->assertArrayHasKey( 'dailyos_account', $post_types );
	}

	/**
	 * The dailyos_account CPT is publicly queryable.
	 */
	public function test_dailyos_account_is_public(): void {
		DailyOS_Plugin::instance()->register_post_types();

		$cpt = $GLOBALS['dailyos_test_registered_post_types']['dailyos_account'] ?? null;

		$this->assertNotNull( $cpt );
		$this->assertTrue( (bool) $cpt->public );
	}

	/**
	 * The dailyos_account CPT exposes a public archive at /accounts/.
	 */
	public function test_dailyos_account_has_archive_and_accounts_slug(): void {
		DailyOS_Plugin::instance()->register_post_types();

		$cpt = $GLOBALS['dailyos_test_registered_post_types']['dailyos_account'] ?? null;

		$this->assertNotNull( $cpt );
		$this->assertTrue( (bool) $cpt->has_archive );
		$this->assertIsArray( $cpt->rewrite );
		$this->assertSame( 'accounts', $cpt->rewrite['slug'] ?? null );
	}

	/**
	 * The dailyos_account CPT is exposed through the REST API as /wp-json/wp/v2/accounts.
	 */
	public function test_dailyos_account_is_exposed_in_rest_with_accounts_base(): void {
		DailyOS_Plugin::instance()->register_post_types();

		$cpt = $GLOBALS['dailyos_test_registered_post_types']['dailyos_account'] ?? null;

		$this->assertNotNull( $cpt );
		$this->assertTrue( (bool) $cpt->show_in_rest );
		$this->assertSame( 'accounts', $cpt->rest_base );
	}
}
