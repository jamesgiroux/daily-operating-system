<?php
/**
 * DailyOS MCP audit tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\Mcp\DailyOS_Mcp_Audit;
use PHPUnit\Framework\Attributes\DataProvider;
use PHPUnit\Framework\TestCase;

/**
 * Verifies MCP audit event validation.
 */
final class DailyOS_McpAuditTest extends TestCase {
	/**
	 * Reset WordPress test doubles.
	 */
	protected function setUp(): void {
		parent::setUp();
		dailyos_test_reset_globals();
	}

	/**
	 * Missing actor instance rejects the audit event.
	 */
	public function test_missing_actor_instance_fails(): void {
		$event = $this->valid_event();
		unset( $event['actor_instance'] );

		$this->expectException( \InvalidArgumentException::class );
		$this->expectExceptionMessage( 'Missing MCP audit event key.' );

		DailyOS_Mcp_Audit::emit( $event );
	}

	/**
	 * Invalid actor instance rejects the audit event.
	 *
	 * @param mixed $actor_instance Actor instance value.
	 */
	#[DataProvider( 'invalid_actor_instance_provider' )]
	public function test_invalid_actor_instance_fails( mixed $actor_instance ): void {
		$event                   = $this->valid_event();
		$event['actor_instance'] = $actor_instance;

		$this->expectException( \InvalidArgumentException::class );
		$this->expectExceptionMessage( 'Invalid MCP audit actor instance.' );

		DailyOS_Mcp_Audit::emit( $event );
	}

	/**
	 * Valid audit event emits through the WordPress action.
	 */
	public function test_valid_audit_event_emits(): void {
		$event = $this->valid_event();

		DailyOS_Mcp_Audit::emit( $event );

		$this->assertSame( [ $event ], $GLOBALS['dailyos_test_audit_events'] );
	}

	/**
	 * Invalid actor instance values.
	 *
	 * @return array<string, array{0: mixed}>
	 */
	public static function invalid_actor_instance_provider(): array {
		return [
			'empty string' => [ '' ],
			'whitespace'   => [ " \t\n" ],
			'null'         => [ null ],
			'integer'      => [ 42 ],
		];
	}

	/**
	 * Create a valid MCP audit event.
	 *
	 * @return array<string, mixed>
	 */
	private function valid_event(): array {
		return [
			'mcp_exposure_path'  => DailyOS_Mcp_Audit::EXPOSURE_INVOCABLE,
			'actor_instance'     => 'plugin-1',
			'wp_user_id'         => 42,
			'ability_name'       => 'dailyos/account-overview',
			'scope_check_result' => 'allowed',
		];
	}
}
