/**
 * DailyOS Blockquote Enhancement
 * Transform blockquotes into styled callout boxes
 *
 * Supports callout types via prefixes:
 * - why: / [why] - Default note style
 * - tip: / [tip] - Success/green style
 * - info: / [info] - Info/blue style
 * - warning: / [warning] - Warning/gold style
 *
 * @module enhancements/blockquotes
 */

const EnhanceBlockquotes = {
  /**
   * Transform blockquotes into callout boxes
   * @param {Element} container
   */
  apply(container) {
    const blockquotes = container.querySelectorAll('blockquote');

    blockquotes.forEach(quote => {
      const text = quote.textContent.toLowerCase();
      let label = 'Note';
      let boxClass = 'callout-box';

      // Detect type from content
      if (text.startsWith('why:') || text.includes('[why]')) {
        label = 'Why';
        quote.innerHTML = quote.innerHTML.replace(/^why:\s*/i, '').replace(/\[why\]\s*/i, '');
      } else if (text.startsWith('tip:') || text.includes('[tip]')) {
        label = 'Tip';
        boxClass += ' callout-box-success';
        quote.innerHTML = quote.innerHTML.replace(/^tip:\s*/i, '').replace(/\[tip\]\s*/i, '');
      } else if (text.startsWith('info:') || text.includes('[info]')) {
        label = 'Info';
        boxClass += ' callout-box-info';
        quote.innerHTML = quote.innerHTML.replace(/^info:\s*/i, '').replace(/\[info\]\s*/i, '');
      } else if (text.startsWith('warning:') || text.includes('[warning]')) {
        label = 'Warning';
        boxClass += ' callout-box-warning';
        quote.innerHTML = quote.innerHTML.replace(/^warning:\s*/i, '').replace(/\[warning\]\s*/i, '');
      }

      const callout = document.createElement('div');
      callout.className = boxClass;
      callout.innerHTML = `
        <span class="callout-box-label">${label}</span>
        <div class="callout-box-content">${quote.innerHTML}</div>
      `;

      quote.parentNode.replaceChild(callout, quote);
    });
  }
};

// Make available globally
window.EnhanceBlockquotes = EnhanceBlockquotes;
