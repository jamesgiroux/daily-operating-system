# DailyOS Slide System

A standalone HTML presentation system that carries the DailyOS editorial magazine aesthetic into self-contained slide decks. No build step, no React, no framework dependencies.

**Output:** A single `.html` file per presentation. Opens in any browser. Navigates with keyboard. Prints to clean PDF.

## Quick Start

Every presentation follows this structure:

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Presentation Title — DailyOS Slides</title>
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=DM+Sans:ital,opsz,wght@0,9..40,300..700;1,9..40,300..700&family=JetBrains+Mono:wght@400;500;600&family=Montserrat:wght@800&family=Newsreader:ital,opsz,wght@0,6..72,300;0,6..72,400;0,6..72,500;0,6..72,600;1,6..72,300;1,6..72,400&display=swap" rel="stylesheet">
  <style>
    /* Paste the full contents of dailyos-slides.css here */
  </style>
</head>
<body>
  <div class="slide-container" id="slides">
    <!-- Slides go here -->
  </div>
  <div class="slide-progress" id="progress"></div>
  <script>
  (function() {
    var container = document.getElementById('slides');
    var slides = container.querySelectorAll('.slide');
    var progress = document.getElementById('progress');
    var total = slides.length;
    function currentSlideIndex() {
      var scrollY = window.scrollY;
      var closest = 0, minDist = Infinity;
      slides.forEach(function(s, i) {
        var dist = Math.abs(s.offsetTop - scrollY);
        if (dist < minDist) { minDist = dist; closest = i; }
      });
      return closest;
    }
    function goTo(index) {
      if (index >= 0 && index < total) slides[index].scrollIntoView({ behavior: 'smooth' });
    }
    function updateProgress() {
      progress.textContent = (currentSlideIndex() + 1) + ' / ' + total;
    }
    document.addEventListener('keydown', function(e) {
      var tag = e.target.tagName;
      if (tag === 'INPUT' || tag === 'TEXTAREA' || e.target.isContentEditable) return;
      var idx = currentSlideIndex();
      if (e.key === 'ArrowDown' || e.key === 'ArrowRight') { e.preventDefault(); goTo(idx + 1); }
      else if (e.key === 'ArrowUp' || e.key === 'ArrowLeft') { e.preventDefault(); goTo(idx - 1); }
      else { var num = parseInt(e.key, 10); if (num >= 1 && num <= 9) { e.preventDefault(); goTo(num - 1); } }
    });
    window.addEventListener('scroll', updateProgress, { passive: true });
    updateProgress();
  })();
  </script>
</body>
</html>
```

## Theme

Set `--accent` in `:root` to change the entire deck's accent color.

| Name | Hex | CSS Variable | Best For |
|------|-----|-------------|----------|
| Turmeric (default) | `#c9a227` | `--color-spice-turmeric` | Account reviews, general |
| Larkspur | `#8fa3c4` | `--color-garden-larkspur` | People, EBR/QBR, informational |
| Terracotta | `#c4654a` | `--color-spice-terracotta` | Risk briefings, urgency |
| Sage | `#7eaa7b` | `--color-garden-sage` | Success stories, health |
| Olive | `#6b7c52` | `--color-garden-olive` | Projects, operational |
| Eucalyptus | `#6ba8a4` | `--color-garden-eucalyptus` | Personal, impact reports |

To apply: change `--accent` and `--accent-tint` in `:root`:
```css
:root {
  --accent: #8fa3c4;                      /* larkspur */
  --accent-tint: rgba(143, 163, 196, 0.05); /* larkspur at 5% */
}
```

## Block Catalog

### 1. Cover

Full-viewport hero slide. Use for the opening of every presentation.

```html
<section class="slide slide--cover">
  <div class="overline">Overline Label</div>
  <h1 class="cover__title">Presentation Title</h1>
  <p class="cover__subtitle">Subtitle or thesis statement.</p>
  <p class="cover__narrative">1-2 sentences of context.</p>
  <div class="cover__meta">
    <span>March 17, 2026</span>
    <span class="cover__meta-divider">/</span>
    <span>Author Name</span>
  </div>
</section>
```

### 2. Section Divider

Chapter heading. Use to introduce a new major section.

```html
<section class="slide slide--section">
  <div class="section__rule"></div>
  <div class="section__number">Section 01</div>
  <h2 class="section__title">Section Title</h2>
  <p class="section__epigraph">Optional italic context line.</p>
</section>
```

### 3. Narrative

Prose block. Can be a full slide or embedded within other slides.

```html
<section class="slide slide--narrative">
  <div class="overline">Context</div>
  <p class="narrative narrative--large">Large opening statement.</p>
  <p class="narrative mt-lg">Follow-up paragraph at regular size.</p>
</section>
```

Variants: `narrative--large` (24px serif), `narrative--sans` (15px DM Sans, secondary color).

### 4. Bullet List

Editorial bullets with generous spacing. Use for 3-5 key points.

```html
<section class="slide">
  <div class="overline">Key Points</div>
  <div class="bullet-list">
    <div class="bullet-list__item">
      <span class="bullet-list__dot">&middot;</span>
      <span class="bullet-list__text"><strong>Lead-in.</strong> Detail.</span>
    </div>
    <!-- Repeat for each item -->
  </div>
</section>
```

Variant: Add `bullet-list--compact` to the container for denser lists (16px sans).

### 5. Two-Column

Side-by-side content. Use for comparisons, pros/cons, or paired content.

```html
<section class="slide">
  <div class="overline">Comparison</div>
  <div class="two-column">
    <div class="two-column__col">
      <div class="two-column__heading two-column__heading--sage">Left Heading</div>
      <div class="two-column__item">
        <span class="two-column__dot two-column__dot--sage"></span>
        <span>Item text.</span>
      </div>
    </div>
    <div class="two-column__col">
      <div class="two-column__heading two-column__heading--terracotta">Right Heading</div>
      <div class="two-column__item">
        <span class="two-column__dot two-column__dot--terracotta"></span>
        <span>Item text.</span>
      </div>
    </div>
  </div>
</section>
```

Heading colors: `--sage`, `--terracotta`, `--accent`, `--larkspur`. Dot colors match.

### 6. Quote Block

Pull quote for key statements. Use for memorable phrases or recommended language.

```html
<!-- Centered with rule -->
<div class="quote-block">
  <div class="quote-block__rule"></div>
  <p class="quote-block__text">"Quote text here."</p>
  <div class="quote-block__attribution">Source</div>
</div>

<!-- Left-aligned with accent border -->
<div class="quote-block quote-block--left">
  <p class="quote-block__text">"Quote text here."</p>
  <div class="quote-block__attribution">Source</div>
</div>
```

### 7. Callout Box

Highlighted box for warnings, tips, notes. Use for "don't do this" or important context.

```html
<div class="callout callout--warning">
  <div class="callout__label">Caution</div>
  <div class="callout__text">Warning text.</div>
</div>
```

Variants: `callout--warning` (terracotta), `callout--tip` (sage), `callout--note` (larkspur), no modifier (accent).

### 8. Story Card

Outcome with impact and source. Use for customer stories, case studies.

```html
<div class="story-cards">
  <div class="story-card">
    <div class="story-card__title">What happened.</div>
    <div class="story-card__impact">Key metric or outcome</div>
    <div class="story-card__source">Source context</div>
  </div>
  <!-- Repeat, 2-4 cards per slide -->
</div>
```

### 9. Data Table

CSS grid table. Use for reference data, comparison matrices.

```html
<div class="data-table">
  <div class="data-table__header" style="grid-template-columns: 1fr 120px 100px 1fr;">
    <div class="data-table__header-cell">Name</div>
    <div class="data-table__header-cell">Value</div>
    <div class="data-table__header-cell">Trend</div>
    <div class="data-table__header-cell">Notes</div>
  </div>
  <div class="data-table__row" style="grid-template-columns: 1fr 120px 100px 1fr;">
    <div class="data-table__cell">Item</div>
    <div class="data-table__cell data-table__cell--mono">$1.2M</div>
    <div class="data-table__cell trend--up">↑ 12%</div>
    <div class="data-table__cell data-table__cell--secondary">Detail</div>
  </div>
</div>
```

Set `grid-template-columns` on both header and rows. Cell modifiers: `--mono`, `--secondary`, `--accent`, `--wrap`. Trend: `trend--up` (sage), `trend--down` (terracotta), `trend--flat`.

### 10. Metric Highlight

Big numbers with labels. Use for KPIs, stats, at-a-glance data.

```html
<div class="metric-row">
  <div class="metric">
    <div class="metric__value metric__value--accent">47</div>
    <div class="metric__label">Meetings</div>
    <div class="metric__delta metric__delta--positive">+8%</div>
  </div>
  <!-- Repeat, 2-4 metrics per row -->
</div>
```

Value colors: `--accent`, `--sage`, `--terracotta`. Delta: `--positive` (sage), `--negative` (terracotta).

### 11. Three-Up Cards

Side-by-side concept cards. Use for features, pillars, building blocks.

```html
<div class="three-up">
  <div class="three-up__card">
    <div class="three-up__number">01</div>
    <div class="three-up__title">Title</div>
    <div class="three-up__body">Description.</div>
  </div>
  <!-- 2-3 cards -->
</div>
```

### 12. Timeline

Sequential events. Use for project history, roadmaps.

```html
<div class="timeline">
  <div class="timeline__event">
    <div class="timeline__dot"></div>
    <div class="timeline__date">January 2026</div>
    <div class="timeline__text">Event description.</div>
  </div>
  <!-- Repeat -->
</div>
```

### 13. Diagnostic Flow

Routing table. Use for "if X → go to Y" quick reference.

```html
<div class="diagnostic">
  <div class="diagnostic__header">
    <div class="diagnostic__header-cell">If you hear...</div>
    <div class="diagnostic__header-cell">Go to</div>
  </div>
  <div class="diagnostic__row">
    <div class="diagnostic__condition">Condition text</div>
    <div class="diagnostic__action"><span class="diagnostic__arrow">&rarr;</span> Action</div>
  </div>
</div>
```

### 14. Stakeholder Grid

People/team display. Use for attendee lists, team overviews.

```html
<div class="stakeholder-grid">
  <div class="stakeholder">
    <div class="stakeholder__name">Name</div>
    <div class="stakeholder__role">Title</div>
    <div class="stakeholder__focus">Focus areas</div>
  </div>
  <!-- 3-6 people -->
</div>
```

### 15. Progress Bar

Stacked colored bar with legend. Use for portfolio health, completion status.

```html
<div class="progress">
  <div class="progress__label">Label</div>
  <div class="progress__bar">
    <div class="progress__segment progress__segment--sage" style="flex: 60;"></div>
    <div class="progress__segment progress__segment--saffron" style="flex: 25;"></div>
    <div class="progress__segment progress__segment--terracotta" style="flex: 15;"></div>
  </div>
  <div class="progress__legend">
    <div class="progress__legend-item">
      <div class="progress__legend-dot" style="background: var(--color-garden-sage);"></div>
      Healthy (60%)
    </div>
    <!-- Repeat per segment -->
  </div>
</div>
```

### 16. Image + Text

Media alongside prose. Images must be base64-encoded or URL-referenced.

```html
<div class="image-text">
  <div class="image-text__media">
    <img src="..." alt="Description">
  </div>
  <div class="image-text__prose">
    <p class="narrative">Text alongside the image.</p>
  </div>
</div>
```

### 17. Closing

Finis marker. Use as the final slide of every presentation.

```html
<section class="slide slide--closing">
  <div class="closing__marks"><span>*</span><span>*</span><span>*</span></div>
  <div class="closing__message">End of presentation.</div>
  <div class="closing__date">March 17, 2026</div>
</section>
```

### 18. Blank

Freeform canvas with base slide padding.

```html
<section class="slide slide--blank">
  <!-- Any content -->
</section>
```

## Composition Rules

- **Typical deck:** 10-25 slides.
- **Always start** with a Cover slide.
- **Always end** with a Closing slide.
- **Section Dividers** introduce major topics. Don't use them for every slide.
- **Mix block types** within a slide when content is related: Quote + Callout, Narrative + Story Cards.
- **Use `mt-lg` or `mt-xl`** between blocks within a single slide for spacing.
- Each `<section class="slide">` gets its own `id="slide-N"` (sequential).

## Content Mapping Guide

When converting a markdown document to slides:

| Markdown Pattern | Slide Block |
|---|---|
| `# Title` | **Cover** (slide 1) or **Section Divider** |
| `## Heading` | **Section Divider** |
| Paragraph text | **Narrative** |
| Bullet list (3-5 items) | **Bullet List** |
| Bullet list (6+ items) | **Bullet List compact** or split across slides |
| `> Blockquote` | **Quote Block** |
| Markdown table | **Data Table** |
| Numbered items with descriptions | **Three-Up Cards** or **Bullet List** |
| Warning/note/aside | **Callout Box** |
| If/then routing | **Diagnostic Flow** |
| People with roles | **Stakeholder Grid** |
| Numbers/stats | **Metric Highlight** |
| Sequential events | **Timeline** |
| Two opposing lists | **Two-Column** |
| Case study/example | **Story Card** |

## Keyboard Navigation

| Key | Action |
|-----|--------|
| `↓` or `→` | Next slide |
| `↑` or `←` | Previous slide |
| `1`-`9` | Jump to slide by number |

## Print / PDF

`Cmd+P` in browser produces one slide per page. The `@media print` styles remove scroll-snap, set `page-break-after: always`, and hide the progress indicator.

## Files

```
templates/slides/
├── dailyos-slides.css        ← Full CSS (inline into <style> for production)
├── slide-shell.html          ← Skeleton template with test content
├── README.md                 ← This file
├── blocks/                   ← HTML snippet per block type (18 files)
└── examples/
    └── block-showcase.html       ← Demo: 14-slide QBR showcasing every block type
```
