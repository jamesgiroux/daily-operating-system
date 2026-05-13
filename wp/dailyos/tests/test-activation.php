<?php
/**
 * DailyOS activation smoke tests.
 *
 * @package DailyOS
 */

declare(strict_types=1);

use DailyOS\DailyOS_Plugin;
use PHPUnit\Framework\TestCase;

/**
 * Smoke coverage for plugin bootstrap.
 */
final class DailyOS_Activation_Test extends TestCase {
	/**
	 * Singleton returns the same instance.
	 */
	public function test_plugin_singleton_returns_same_instance(): void {
		$this->assertSame( DailyOS_Plugin::instance(), DailyOS_Plugin::instance() );
	}

	/**
	 * Plugin constants are defined.
	 */
	public function test_plugin_constants_are_defined(): void {
		$this->assertTrue( defined( 'DAILYOS_PLUGIN_FILE' ) );
		$this->assertTrue( defined( 'DAILYOS_PLUGIN_DIR' ) );
		$this->assertTrue( defined( 'DAILYOS_PLUGIN_URL' ) );
		$this->assertTrue( defined( 'DAILYOS_VERSION' ) );
		$this->assertSame( '0.1.0', DAILYOS_VERSION );
	}
}
