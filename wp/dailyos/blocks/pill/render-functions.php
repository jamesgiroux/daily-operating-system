<?php
/**
 * Pill (translated from src/components/ui/Pill.tsx).
 * Variants: tone (sage|turmeric|terracotta|larkspur|neutral),
 *           size (standard|compact), dot (bool), interactive (bool).
 */

declare(strict_types=1);

function dailyos_pill_render(array $attributes): string {
	$tone = isset($attributes['tone']) ? (string) $attributes['tone'] : 'neutral';
	$size = isset($attributes['size']) ? (string) $attributes['size'] : 'standard';
	$dot = !empty($attributes['dot']);
	$interactive = !empty($attributes['interactive']);
	$label = isset($attributes['label']) ? (string) $attributes['label'] : '';

	$allowed_tones = ['sage', 'turmeric', 'terracotta', 'larkspur', 'neutral'];
	if (!in_array($tone, $allowed_tones, true)) {
		$tone = 'neutral';
	}
	$size = ($size === 'compact') ? 'compact' : 'standard';

	$classes = ['dailyos-pill', 'dailyos-pill--tone-' . $tone, 'dailyos-pill--size-' . $size];
	if ($interactive) { $classes[] = 'dailyos-pill--interactive'; }

	$dot_html = $dot ? '<span class="dailyos-pill__dot" aria-hidden="true"></span>' : '';
	$label_html = esc_html($label);

	return sprintf(
		'<span class="%s" data-tone="%s" data-ds-name="Pill" data-ds-tier="primitive" data-ds-spec="primitives/Pill.md">%s%s</span>',
		esc_attr(implode(' ', $classes)),
		esc_attr($tone),
		$dot_html,
		$label_html
	);
}
