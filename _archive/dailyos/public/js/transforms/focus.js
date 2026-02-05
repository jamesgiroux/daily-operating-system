/**
 * DailyOS Focus Page Transform
 * Transforms the focus page into a card-based layout
 *
 * @module transforms/focus
 */

const FocusTransform = {
  /**
   * Transform name for registry identification
   * @type {string}
   */
  name: 'focus',

  /**
   * Detect if this is a focus page
   * @param {Element} container - DOM container to check
   * @returns {boolean} True if this is a focus page
   */
  detect(container) {
    const h1 = container.querySelector('h1');
    return h1 && (h1.textContent.includes('Suggested Focus') || h1.textContent.includes('Focus Areas'));
  },

  /**
   * Apply the focus page transformation
   * @param {Element} container - DOM container to transform
   */
  apply(container) {
    const h1 = container.querySelector('h1');
    const dateMatch = h1.textContent.match(/- (.+)/);
    const dateStr = dateMatch ? dateMatch[1] : 'Focus Areas';

    // Build dashboard
    const dashboard = document.createElement('div');
    dashboard.className = 'dashboard';

    dashboard.innerHTML = `
      <div class="dashboard-header">
        <div>
          <h1 class="dashboard-title">Suggested Focus</h1>
          <p class="dashboard-subtitle">${dateStr}</p>
        </div>
      </div>
    `;

    // Find priority sections
    const sections = this.findSections(container);

    const mainContent = document.createElement('div');
    mainContent.className = 'focus-layout';

    // Build focus sections
    let priorityNum = 1;
    for (const key of ['p1', 'p2', 'p3', 'p4', 'p5']) {
      if (sections[key]) {
        mainContent.appendChild(this.buildFocusSection(sections[key], priorityNum, 0.2 + priorityNum * 0.05));
        priorityNum++;
      }
    }

    if (sections.energy) {
      mainContent.appendChild(this.buildEnergySection(sections.energy, 0.5));
    }

    dashboard.appendChild(mainContent);
    container.innerHTML = '';
    container.appendChild(dashboard);
  },

  /**
   * Find sections in the focus page
   * @param {Element} container - Container to search
   * @returns {Object} Map of section names to heading elements
   */
  findSections(container) {
    const sections = {};
    const h2s = container.querySelectorAll('h2');
    h2s.forEach(h2 => {
      const text = h2.textContent.toLowerCase();
      if (text.includes('priority 1')) sections.p1 = h2;
      if (text.includes('priority 2')) sections.p2 = h2;
      if (text.includes('priority 3')) sections.p3 = h2;
      if (text.includes('priority 4')) sections.p4 = h2;
      if (text.includes('priority 5')) sections.p5 = h2;
      if (text.includes('energy')) sections.energy = h2;
    });
    return sections;
  },

  /**
   * Build focus section card
   * @param {Element} heading - Section heading
   * @param {number} priorityNum - Priority number (1-5)
   * @param {number} delay - Animation delay
   * @returns {HTMLElement} Focus card element
   */
  buildFocusSection(heading, priorityNum, delay) {
    const section = document.createElement('div');
    section.className = 'focus-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const title = heading.textContent;
    const content = SectionUtils.findContent(heading);
    let itemsHtml = '';

    content.forEach(el => {
      if (el.tagName === 'UL') {
        const items = el.querySelectorAll(':scope > li');
        items.forEach((li, i) => {
          const hasCheckbox = li.querySelector('input[type="checkbox"]');
          const text = li.innerHTML.replace(/<input[^>]*>/g, '').trim();

          itemsHtml += `
            <div class="focus-item">
              ${hasCheckbox ? '<input type="checkbox" class="focus-checkbox" />' : `<span class="focus-number">${i + 1}</span>`}
              <div class="focus-item-text">${text}</div>
            </div>
          `;
        });
      }

      if (el.tagName === 'P') {
        itemsHtml += `<p class="focus-note">${el.innerHTML}</p>`;
      }
    });

    section.innerHTML = `
      <div class="focus-card-header">
        <span class="focus-priority-badge p${priorityNum}">P${priorityNum}</span>
        <h3 class="focus-card-title">${title.replace(/Priority \d+:?\s*/i, '')}</h3>
      </div>
      <div class="focus-card-body">
        ${itemsHtml || '<p class="text-muted">No items</p>'}
      </div>
    `;

    return section;
  },

  /**
   * Build energy awareness section
   * @param {Element} heading - Energy section heading
   * @param {number} delay - Animation delay
   * @returns {HTMLElement} Energy section element
   */
  buildEnergySection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const content = SectionUtils.findContent(heading);
    let itemsHtml = '';

    content.forEach(el => {
      if (el.tagName === 'UL') {
        const items = el.querySelectorAll('li');
        items.forEach(li => {
          itemsHtml += `<li>${li.innerHTML}</li>`;
        });
      }
    });

    section.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: 8px;">
            <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"></polygon>
          </svg>
          Energy Awareness
        </h3>
      </div>
      <div class="section-card-body">
        <ul class="energy-list">${itemsHtml}</ul>
      </div>
    `;

    return section;
  }
};

// Register with TransformRegistry
if (window.TransformRegistry) {
  TransformRegistry.register(FocusTransform);
}

// Make available globally
window.FocusTransform = FocusTransform;
