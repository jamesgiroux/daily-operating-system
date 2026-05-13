<?php
/**
 * DailyOS runtime client contract notes.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS;

/**
 * Documents the W3-B PHP-to-runtime transport boundary.
 *
 * The wp_remote_post body MUST be a string, not an array — see W3-0 spike
 * byte-exactness test caveats.
 */
final class DailyOS_Runtime_Client_Contract {
	/**
	 * Static contract holder.
	 */
	private function __construct() {}
}
