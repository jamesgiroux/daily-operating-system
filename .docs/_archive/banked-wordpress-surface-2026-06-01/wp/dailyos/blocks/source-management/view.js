( function () {
	function endpoint() {
		if ( window.wpApiSettings && window.wpApiSettings.root ) {
			return window.wpApiSettings.root.replace( /\/$/, '' ) + '/dailyos/v1/source-management/action';
		}
		return '/wp-json/dailyos/v1/source-management/action';
	}

	function nonce() {
		return window.wpApiSettings && window.wpApiSettings.nonce ? window.wpApiSettings.nonce : '';
	}

	async function applyAction( button ) {
		const payload = {
			action: button.dataset.dailyosSourceAction || '',
			sourceKey: button.dataset.dailyosSourceKey || '',
			entityType: button.dataset.dailyosEntityType || '',
			entityId: button.dataset.dailyosEntityId || '',
		};
		button.disabled = true;
		button.setAttribute( 'aria-busy', 'true' );
		try {
			const response = await window.fetch( endpoint(), {
				method: 'POST',
				credentials: 'same-origin',
				headers: {
					'Content-Type': 'application/json',
					'X-WP-Nonce': nonce(),
				},
				body: JSON.stringify( payload ),
			} );
			if ( ! response.ok ) {
				throw new Error( 'source_action_failed' );
			}
			button.dispatchEvent(
				new CustomEvent( 'dailyos:source-management-action', {
					bubbles: true,
					detail: payload,
				} )
			);
		} catch ( error ) {
			button.disabled = false;
			button.dataset.dailyosActionError = 'true';
		} finally {
			button.removeAttribute( 'aria-busy' );
		}
	}

	document.addEventListener( 'click', function ( event ) {
		const button = event.target.closest( '[data-dailyos-source-action]' );
		if ( ! button || button.disabled ) {
			return;
		}
		event.preventDefault();
		applyAction( button );
	} );
} )();
