<?php
/**
 * Final render-boundary sanitizer for DailyOS markdown preview HTML.
 *
 * @package DailyOS
 */

declare(strict_types=1);

namespace DailyOS;

/**
 * Sanitizes already-rendered preview HTML before it reaches the browser.
 */
final class DailyOS_Markdown_Sanitizer {
	// phpcs:disable WordPress.NamingConventions.ValidVariableName.UsedPropertyNotSnakeCase -- DOMDocument exposes camelCase properties.
	public const LOCAL_ASSET_PLACEHOLDER = '[local asset blocked]';
	public const REMOTE_ASSET_PLACEHOLDER = '[remote asset blocked]';

	/**
	 * Tags whose full subtree is discarded.
	 *
	 * @var array<string, true>
	 */
	private const DROP_SUBTREE_TAGS = [
		'base'          => true,
		'button'        => true,
		'dialog'        => true,
		'embed'         => true,
		'foreignobject' => true,
		'form'          => true,
		'iframe'        => true,
		'input'         => true,
		'link'          => true,
		'math'          => true,
		'meta'          => true,
		'object'        => true,
		'option'        => true,
		'script'        => true,
		'select'        => true,
		'style'         => true,
		'svg'           => true,
		'textarea'      => true,
	];

	/**
	 * Tags allowed to remain in sanitized output.
	 *
	 * @var array<string, true>
	 */
	private const ALLOWED_TAGS = [
		'a'          => true,
		'blockquote' => true,
		'br'         => true,
		'code'       => true,
		'em'         => true,
		'h1'         => true,
		'h2'         => true,
		'h3'         => true,
		'h4'         => true,
		'h5'         => true,
		'h6'         => true,
		'hr'         => true,
		'img'        => true,
		'li'         => true,
		'ol'         => true,
		'p'          => true,
		'pre'        => true,
		'strong'     => true,
		'table'      => true,
		'tbody'      => true,
		'td'         => true,
		'th'         => true,
		'thead'      => true,
		'tr'         => true,
		'ul'         => true,
	];

	/**
	 * Sanitize an HTML fragment.
	 *
	 * @param string               $html HTML fragment.
	 * @param array<string, mixed> $options Sanitizer options.
	 * @return string
	 */
	public function sanitize( string $html, array $options = [] ): string {
		if ( ! empty( $options['force_fail_closed'] ) || ! class_exists( \DOMDocument::class ) ) {
			return $this->fail_closed( $html );
		}

		$document = new \DOMDocument( '1.0', 'UTF-8' );
		$previous = libxml_use_internal_errors( true );
		$loaded = $document->loadHTML(
			'<?xml encoding="utf-8" ?><!DOCTYPE html><html><body><div id="dailyos-markdown-preview-root">' . $html . '</div></body></html>',
			LIBXML_NOERROR | LIBXML_NOWARNING
		);
		libxml_clear_errors();
		libxml_use_internal_errors( $previous );

		if ( ! $loaded ) {
			return $this->fail_closed( $html );
		}

		$root = $document->getElementById( 'dailyos-markdown-preview-root' );
		if ( null === $root ) {
			return $this->fail_closed( $html );
		}

		$this->sanitize_children( $root, $document, $options );

		$out = '';
		foreach ( iterator_to_array( $root->childNodes ) as $child ) {
			$out .= $document->saveHTML( $child );
		}

		return trim( $out );
	}

	/**
	 * Escape the whole fragment when structural sanitization is unavailable.
	 *
	 * @param string $html Original fragment.
	 * @return string
	 */
	public function fail_closed( string $html ): string {
		return '<div class="dailyos-markdown-preview__fallback" data-sanitizer-state="fail_closed">'
			. htmlspecialchars( $html, ENT_QUOTES | ENT_SUBSTITUTE, 'UTF-8' )
			. '</div>';
	}

	/**
	 * Sanitize every child under a parent node.
	 *
	 * @param \DOMNode             $parent Parent node.
	 * @param \DOMDocument         $document DOM document.
	 * @param array<string, mixed> $options Sanitizer options.
	 */
	private function sanitize_children( \DOMNode $parent, \DOMDocument $document, array $options ): void {
		for ( $node = $parent->firstChild; null !== $node; ) {
			$next = $node->nextSibling;
			$this->sanitize_node( $node, $document, $options );
			$node = $next;
		}
	}

	/**
	 * Sanitize one node.
	 *
	 * @param \DOMNode             $node Node.
	 * @param \DOMDocument         $document DOM document.
	 * @param array<string, mixed> $options Sanitizer options.
	 */
	private function sanitize_node( \DOMNode $node, \DOMDocument $document, array $options ): void {
		if ( XML_COMMENT_NODE === $node->nodeType ) {
			$this->remove_node( $node );
			return;
		}

		if ( XML_TEXT_NODE === $node->nodeType ) {
			return;
		}

		if ( XML_ELEMENT_NODE !== $node->nodeType || ! $node instanceof \DOMElement ) {
			$this->remove_node( $node );
			return;
		}

		$tag = strtolower( $node->tagName );
		if ( isset( self::DROP_SUBTREE_TAGS[ $tag ] ) || str_contains( $tag, '-' ) ) {
			$this->remove_node( $node );
			return;
		}

		if ( 'img' === $tag ) {
			$this->sanitize_image( $node, $document, $options );
			return;
		}

		if ( ! isset( self::ALLOWED_TAGS[ $tag ] ) ) {
			$this->remove_node( $node );
			return;
		}

		$this->sanitize_attributes( $node );
		$this->sanitize_children( $node, $document, $options );
	}

	/**
	 * Sanitize or replace an image.
	 *
	 * @param \DOMElement          $node Image node.
	 * @param \DOMDocument         $document DOM document.
	 * @param array<string, mixed> $options Sanitizer options.
	 */
	private function sanitize_image( \DOMElement $node, \DOMDocument $document, array $options ): void {
		$src = $node->getAttribute( 'src' );
		if ( ! $this->is_allowed_asset_src( $src, ! empty( $options['allow_dailyos_asset_images'] ) ) ) {
			$placeholder = $this->is_dailyos_asset_src( $src ) ? self::LOCAL_ASSET_PLACEHOLDER : self::REMOTE_ASSET_PLACEHOLDER;
			$this->replace_with_text( $node, $document, $placeholder );
			return;
		}

		$this->sanitize_attributes( $node );
	}

	/**
	 * Sanitize allowed-element attributes.
	 *
	 * @param \DOMElement $element Element.
	 */
	private function sanitize_attributes( \DOMElement $element ): void {
		foreach ( iterator_to_array( $element->attributes ) as $attribute ) {
			$name = strtolower( $attribute->name );
			$value = $attribute->value;

			if ( str_starts_with( $name, 'on' ) ) {
				$element->removeAttribute( $attribute->name );
				continue;
			}

			switch ( $name ) {
				case 'href':
					if ( 'a' === strtolower( $element->tagName ) && $this->is_allowed_href( $value ) ) {
						$element->setAttribute( 'href', $this->clean_attribute_text( $value ) );
						$element->setAttribute( 'rel', 'nofollow noopener noreferrer' );
					} else {
						$element->removeAttribute( $attribute->name );
					}
					break;
				case 'src':
					if ( 'img' !== strtolower( $element->tagName ) || ! $this->is_allowed_asset_src( $value, true ) ) {
						$element->removeAttribute( $attribute->name );
					}
					break;
				case 'alt':
					if ( 'img' === strtolower( $element->tagName ) ) {
						$element->setAttribute( 'alt', $this->clean_attribute_text( $value ) );
					} else {
						$element->removeAttribute( $attribute->name );
					}
					break;
				case 'title':
					$element->setAttribute( 'title', $this->clean_attribute_text( $value ) );
					break;
				case 'lang':
					if ( 1 === preg_match( '/^[a-z]{2,8}(?:-[a-z0-9]{1,8})*$/i', $value ) ) {
						$element->setAttribute( 'lang', strtolower( $value ) );
					} else {
						$element->removeAttribute( $attribute->name );
					}
					break;
				case 'dir':
					if ( in_array( strtolower( $value ), [ 'ltr', 'rtl', 'auto' ], true ) ) {
						$element->setAttribute( 'dir', strtolower( $value ) );
					} else {
						$element->removeAttribute( $attribute->name );
					}
					break;
				case 'class':
					$class = $this->clean_class_list( $value );
					if ( '' === $class ) {
						$element->removeAttribute( $attribute->name );
					} else {
						$element->setAttribute( 'class', $class );
					}
					break;
				case 'rel':
					if ( 'a' !== strtolower( $element->tagName ) || ! $element->hasAttribute( 'href' ) ) {
						$element->removeAttribute( $attribute->name );
					}
					break;
				default:
					$element->removeAttribute( $attribute->name );
					break;
			}
		}
	}

	/**
	 * Check whether a link href is display-safe.
	 *
	 * @param string $href Href value.
	 * @return bool
	 */
	private function is_allowed_href( string $href ): bool {
		$value = trim( html_entity_decode( $href, ENT_QUOTES | ENT_HTML5, 'UTF-8' ) );
		if ( '' === $value || str_starts_with( $value, '//' ) || 1 === preg_match( '/[\x00-\x1F\x7F]/', $value ) ) {
			return false;
		}

		$scheme = wp_parse_url( $value, PHP_URL_SCHEME );
		return is_string( $scheme ) && in_array( strtolower( $scheme ), [ 'http', 'https', 'mailto' ], true );
	}

	/**
	 * Check whether an image src is a resolvable DailyOS asset URI.
	 *
	 * @param string $src Source value.
	 * @param bool   $allow_assets Whether asset image rendering is enabled.
	 * @return bool
	 */
	private function is_allowed_asset_src( string $src, bool $allow_assets ): bool {
		return $allow_assets && $this->is_dailyos_asset_src( $src );
	}

	/**
	 * Check whether an image src is a DailyOS asset URI.
	 *
	 * @param string $src Source value.
	 * @return bool
	 */
	private function is_dailyos_asset_src( string $src ): bool {
		$value = trim( html_entity_decode( $src, ENT_QUOTES | ENT_HTML5, 'UTF-8' ) );
		return str_starts_with( strtolower( $value ), 'dailyos-asset://' )
			&& 0 === preg_match( '/[\x00-\x1F\x7F]/', $value );
	}

	/**
	 * Clean an attribute text value.
	 *
	 * @param string $value Attribute value.
	 * @return string
	 */
	private function clean_attribute_text( string $value ): string {
		$text = trim( html_entity_decode( $value, ENT_QUOTES | ENT_HTML5, 'UTF-8' ) );
		return preg_replace( '/[\x00-\x1F\x7F]/', '', $text ) ?? '';
	}

	/**
	 * Keep only markdown-preview namespaced CSS classes.
	 *
	 * @param string $value Class attribute.
	 * @return string
	 */
	private function clean_class_list( string $value ): string {
		$tokens = preg_split( '/\s+/', trim( $value ) );
		if ( false === $tokens ) {
			$tokens = [];
		}
		$allowed = [];
		foreach ( $tokens as $token ) {
			if ( 1 === preg_match( '/^dailyos-markdown-preview(?:__(?:[a-z0-9_-]+)|--(?:[a-z0-9_-]+))?$/', $token ) ) {
				$allowed[] = $token;
			}
		}
		return implode( ' ', array_unique( $allowed ) );
	}

	/**
	 * Replace a node with a text node.
	 *
	 * @param \DOMNode     $node Node to replace.
	 * @param \DOMDocument $document DOM document.
	 * @param string       $text Text replacement.
	 */
	private function replace_with_text( \DOMNode $node, \DOMDocument $document, string $text ): void {
		$node->parentNode?->replaceChild( $document->createTextNode( $text ), $node );
	}

	/**
	 * Remove a node if it still has a parent.
	 *
	 * @param \DOMNode $node Node.
	 */
	private function remove_node( \DOMNode $node ): void {
		$node->parentNode?->removeChild( $node );
	}
}
