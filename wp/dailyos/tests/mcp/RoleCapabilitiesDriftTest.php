<?php
/**
 * DailyOS MCP role capability drift tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\Mcp\DailyOS_Mcp_Roles;
use PHPUnit\Framework\TestCase;

/**
 * Ensures the role capability contract stays pinned.
 */
final class DailyOS_RoleCapabilitiesDriftTest extends TestCase {
	/**
	 * Role capabilities match the pinned fixture.
	 */
	public function test_role_capabilities_match_fixture(): void {
		$fixture = $this->load_fixture();

		$this->assertSame( $fixture['capabilities'], DailyOS_Mcp_Roles::capabilities() );
		$this->assertSame( $fixture['role'], DailyOS_Mcp_Roles::ROLE_SLUG );
	}

	/**
	 * Load the role capability fixture.
	 *
	 * @return array{role: string, capabilities: array<string, bool>}
	 */
	private function load_fixture(): array {
		$path = dirname( __DIR__ ) . '/fixtures/role-capabilities.json';

		// phpcs:ignore WordPress.WP.AlternativeFunctions.file_get_contents_file_get_contents -- Test fixture read.
		return json_decode( (string) file_get_contents( $path ), true, 512, JSON_THROW_ON_ERROR );
	}
}
