/**
 * Dynamic block — save returns null so all output comes from render.php
 * (AC §12).
 */
( function () {
	if ( window.wp && window.wp.blocks && window.wp.blocks.registerBlockType ) {
		// edit.js owns registration; save is the canonical no-op for
		// dynamic blocks.
	}
} )();

// CommonJS export for static analysis; runtime registration happens in
// edit.js so the editor + front-end stay aligned.
module.exports = function save() {
	return null;
};
