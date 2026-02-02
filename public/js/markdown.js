/**
 * DailyOS Markdown Utilities
 * Slim orchestrator for markdown enhancement and page transforms
 *
 * This module coordinates:
 * - Page-specific transforms via TransformRegistry
 * - Generic enhancements via Enhancements module
 * - Visual transformations for alerts, schedules, and animations
 *
 * @module markdown
 */

const MarkdownUtils = {
  /**
   * Enhance rendered HTML with interactive features and visual transformations
   * @param {Element} container - DOM container with rendered markdown
   */
  enhance(container) {
    // Try page-specific transforms first (via registry)
    if (window.TransformRegistry && TransformRegistry.apply(container)) {
      return; // Transform handled the page
    }

    // Apply default enhancements for generic pages
    this.applyDefaultEnhancements(container);

    // Visual transformations
    this.transformAlertSections(container);
    this.transformScheduleTables(container);

    // Apply entrance animations
    this.addEntranceAnimations(container);
  },

  /**
   * Apply default enhancements using modular enhancement system
   * @param {Element} container - DOM container
   */
  applyDefaultEnhancements(container) {
    if (window.Enhancements) {
      Enhancements.applyAll(container);
    }
  },

  /**
   * Parse frontmatter display
   * @param {Object} data - Frontmatter data object
   * @returns {string} HTML string for frontmatter tags
   */
  renderFrontmatter(data) {
    if (!data || Object.keys(data).length === 0) return '';

    const items = [];

    if (data.date) {
      items.push(`<span class="tag">${this.formatDate(data.date)}</span>`);
    }
    if (data.status) {
      const statusClass = data.status === 'active' ? 'tag-green' :
                         data.status === 'draft' ? 'tag-gold' : '';
      items.push(`<span class="tag ${statusClass}">${data.status}</span>`);
    }
    if (data.account) {
      items.push(`<span class="tag tag-gold">${data.account}</span>`);
    }
    if (data.meeting_type) {
      const typeClass = data.meeting_type === 'customer_facing' ? 'tag-customer' :
                       data.meeting_type === 'internal' ? 'tag-internal' : '';
      items.push(`<span class="tag ${typeClass}">${data.meeting_type.replace('_', ' ')}</span>`);
    }
    if (data.tags && Array.isArray(data.tags)) {
      data.tags.slice(0, 3).forEach(tag => {
        items.push(`<span class="tag">${tag}</span>`);
      });
    }

    if (items.length === 0) return '';

    return `<div class="frontmatter-tags" style="display: flex; flex-wrap: wrap; gap: var(--space-2); margin-bottom: var(--space-6);">${items.join('')}</div>`;
  },

  /**
   * Format date nicely
   * @param {string} dateStr - Date string in YYYY-MM-DD format
   * @returns {string} Formatted date string
   */
  formatDate(dateStr) {
    try {
      const date = new Date(dateStr + 'T00:00:00');
      return date.toLocaleDateString('en-US', {
        weekday: 'short',
        month: 'short',
        day: 'numeric',
        year: 'numeric'
      });
    } catch {
      return dateStr;
    }
  },

  /**
   * Transform alert sections (warnings, success, error) into styled boxes
   * @param {Element} container - DOM container
   */
  transformAlertSections(container) {
    const headings = container.querySelectorAll('h2, h3');

    headings.forEach(heading => {
      const text = heading.textContent.toLowerCase();
      let alertType = null;
      let icon = null;

      if (text.includes('attention') || text.includes('warning') || text.includes('overdue')) {
        alertType = 'alert-warning';
        icon = Constants.ICONS.WARNING;
      } else if (text.includes('success') || text.includes('completed') || text.includes('done')) {
        alertType = 'alert-success';
        icon = Constants.ICONS.SUCCESS;
      } else if (text.includes('error') || text.includes('failed') || text.includes('critical')) {
        alertType = 'alert-error';
        icon = Constants.ICONS.ERROR;
      }

      if (alertType) {
        const content = SectionUtils.findContent(heading);

        if (content.length > 0) {
          const alert = document.createElement('div');
          alert.className = `alert ${alertType}`;
          alert.innerHTML = `
            <div class="alert-icon">${icon}</div>
            <div class="alert-content">
              <div class="alert-title">${heading.textContent}</div>
              <div class="alert-description"></div>
            </div>
          `;

          const descContainer = alert.querySelector('.alert-description');
          content.forEach(el => {
            descContainer.appendChild(el.cloneNode(true));
            el.remove();
          });

          heading.parentNode.insertBefore(alert, heading);
          heading.remove();
        }
      }
    });
  },

  /**
   * Transform schedule tables into preview cards
   * @param {Element} container - DOM container
   */
  transformScheduleTables(container) {
    const tables = container.querySelectorAll('table');

    tables.forEach(table => {
      const headers = Array.from(table.querySelectorAll('th')).map(th => th.textContent.toLowerCase().trim());

      const timeColIndex = headers.findIndex(h => h.includes('time') || h.includes('when') || h.includes('start'));
      const titleColIndex = headers.findIndex(h => h.includes('meeting') || h.includes('event') || h.includes('title') || h.includes('what'));
      const typeColIndex = headers.findIndex(h => h.includes('type') || h.includes('category') || h.includes('tag'));

      if (timeColIndex !== -1 && titleColIndex !== -1) {
        const rows = SectionUtils.getTableRows(table);
        if (rows.length === 0) return;

        const preview = document.createElement('div');
        preview.className = 'output-preview';

        let html = '<div class="output-preview-header">Schedule</div>';

        rows.forEach(row => {
          const cells = row.querySelectorAll('td');
          const time = cells[timeColIndex]?.textContent.trim() || '';
          const title = cells[titleColIndex]?.textContent.trim() || '';
          const type = cells[typeColIndex]?.textContent.toLowerCase().trim() || '';

          const meetingType = SectionUtils.classifyMeetingType(type);
          const tagClass = `meeting-tag meeting-tag-${meetingType}`;

          html += `
            <div class="meeting-row">
              <span class="meeting-time">${time}</span>
              <span class="meeting-name ${meetingType === 'customer' ? 'customer' : ''}">${title}</span>
              <span class="${tagClass}">${meetingType}</span>
            </div>
          `;
        });

        preview.innerHTML = html;

        const wrapper = table.closest('.table-wrapper') || table;
        wrapper.parentNode.replaceChild(preview, wrapper);
      }
    });
  },

  /**
   * Add entrance animations to content elements
   * @param {Element} container - DOM container
   */
  addEntranceAnimations(container) {
    const baseDelay = Constants.ANIMATION.BASE_DELAY;
    const stagger = Constants.ANIMATION.STAGGER;

    // Animate headings
    const headings = container.querySelectorAll('h1, h2, h3');
    headings.forEach((h, i) => {
      h.classList.add('animate-in');
      h.style.animationDelay = `${i * stagger}s`;
    });

    // Animate cards and alerts
    const cards = container.querySelectorAll('.card, .alert, .output-preview, .callout-box, .terminal');
    cards.forEach((card, i) => {
      card.classList.add('animate-in');
      card.style.animationDelay = `${baseDelay + i * 0.08}s`;
    });

    // Animate folder items
    const folderItems = container.querySelectorAll('.folder-item');
    folderItems.forEach((item, i) => {
      item.classList.add('animate-in-fast');
      item.style.animationDelay = `${baseDelay + i * 0.04}s`;
    });

    // Animate file list items
    const fileItems = container.querySelectorAll('.file-list-item');
    fileItems.forEach((item, i) => {
      item.classList.add('animate-in-fast');
      item.style.animationDelay = `${baseDelay + i * 0.03}s`;
    });

    // Animate meeting rows
    const meetingRows = container.querySelectorAll('.meeting-row');
    meetingRows.forEach((row, i) => {
      row.classList.add('animate-in-fast');
      row.style.animationDelay = `${0.15 + i * stagger}s`;
    });
  }
};

// Export for use in other modules
window.MarkdownUtils = MarkdownUtils;
