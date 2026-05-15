<?php
/**
 * WP HTTP API byte-exactness probe for JSON request bodies.
 *
 * Runs inside Studio (or any WP-CLI environment) and POSTs canonical
 * bodies to a receiver running on the host. The receiver records the
 * raw wire bytes; this script writes the input bodies to a sibling
 * directory so the harness can compare sha256(input) vs sha256(wire).
 *
 * Env:
 *   DOS_BYTE_EXACTNESS_HOST   default 127.0.0.1
 *   DOS_BYTE_EXACTNESS_PORT   required (receiver listening port)
 *   DOS_BYTE_EXACTNESS_OUTDIR required (writable from PHP for input dumps)
 *
 * Usage:
 *   studio wp eval-file byte-exactness.php
 */

if ( ! function_exists( 'wp_remote_post' ) ) {
	fwrite( STDERR, "FAIL wp_remote_post() is unavailable. Run via WP-CLI.\n" );
	exit( 2 );
}

// Studio's WASM PHP does not inherit shell env vars; load config from a
// sibling JSON file the host harness writes before invoking wp eval-file.
$config_path = __DIR__ . '/config.json';
if ( ! is_readable( $config_path ) ) {
	fwrite( STDERR, "FAIL config.json not found at $config_path\n" );
	exit( 2 );
}
$config = json_decode( file_get_contents( $config_path ), true );
$host   = $config['host'] ?? '127.0.0.1';
$port   = (int) ( $config['port'] ?? 0 );
$outdir = $config['outdir'] ?? '';

if ( $port < 1 || $port > 65535 ) {
	fwrite( STDERR, "FAIL DOS_BYTE_EXACTNESS_PORT must be 1..65535 (got '$port').\n" );
	exit( 2 );
}
if ( ! $outdir || ! is_dir( $outdir ) || ! is_writable( $outdir ) ) {
	fwrite( STDERR, "FAIL DOS_BYTE_EXACTNESS_OUTDIR must be a writable directory (got '$outdir').\n" );
	exit( 2 );
}

$unicode = "Cafe\u{0301} | \u{6771}\u{4EAC} | \u{2603} | \u{1F680}";
$rtl     = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}";
$cases   = array(
	'leading-trailing-json-whitespace' => " \t\r\n{\r\n  \"edge\" : \"outer JSON whitespace is preserved\",\r\n  \"array\" : [ 1, true, null ],\r\n  \"object\" : { \"spaced\" : \"value\" }\r\n}\r\n\t ",
	'unescaped-utf8-unicode'           => "{\n  \"unicode\" : \"$unicode\",\n  \"rtl\" : \"$rtl\",\n  \"combining\" : \"e\u{0301}\"\n}\n",
	'escaped-control-whitespace'       => "{\n  \"escaped\" : \"tab\\\\t newline\\\\n carriage\\\\r\",\n  \"literal_spaces\" : \"  keep both edges  \",\n  \"slash\" : \"a\\/b\"\n}\n",
	'utf8-bom-and-multibyte'           => "\xEF\xBB\xBF{\n  \"bom\" : \"leading BOM in body\",\n  \"emoji\" : \"\xF0\x9F\x9A\x80 \xE2\x98\x83\",\n  \"two-byte\" : \"\xC3\xA9\xC3\xA8\"\n}\n",
);

$i = 0;
foreach ( $cases as $name => $body ) {
	$i++;
	$path = $outdir . '/case-' . $i . '.input';
	file_put_contents( $path, $body );

	$response = wp_remote_post(
		"http://$host:$port/byte-exactness/$name",
		array(
			'body'        => $body,
			'headers'     => array(
				'Content-Type' => 'application/json; charset=utf-8',
				'X-DOS-Case'   => $name,
			),
			'timeout'     => 5,
			'redirection' => 0,
		)
	);

	$err  = is_wp_error( $response ) ? $response->get_error_message() : '';
	$code = is_wp_error( $response ) ? 0 : (int) wp_remote_retrieve_response_code( $response );
	printf(
		"sent #%d %s len=%d sha256=%s http=%d err=%s\n",
		$i,
		$name,
		strlen( $body ),
		hash( 'sha256', $body ),
		$code,
		$err
	);
}

printf( "done sent=%d\n", $i );
exit( 0 );
