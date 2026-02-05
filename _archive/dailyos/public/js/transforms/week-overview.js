/**
 * DailyOS Week Overview Page Transform
 * Transforms the week overview page into a dashboard layout
 *
 * @module transforms/week-overview
 */

const WeekOverviewTransform = {
  /**
   * Transform name for registry identification
   * @type {string}
   */
  name: 'week-overview',

  /**
   * Detect if this is a week overview page
   * @param {Element} container - DOM container to check
   * @returns {boolean} True if this is a week overview page
   */
  detect(container) {
    const h1 = container.querySelector('h1');
    return h1 && h1.textContent.includes('Week Overview');
  },

  /**
   * Apply the week overview page transformation
   * @param {Element} container - DOM container to transform
   */
  apply(container) {
    const h1 = container.querySelector('h1');
    const titleMatch = h1.textContent.match(/Week Overview:\s*(.+)/);
    const weekTitle = titleMatch ? titleMatch[1] : 'Week Overview';

    // Get focus description from first paragraph
    const firstP = container.querySelector('p');
    const focusText = firstP ? firstP.textContent.replace('Your Focus This Week:', '').trim() : '';

    // Find sections
    const sections = this.findSections(container);

    // Count stats
    const stats = this.countStats(container, sections);

    // Build dashboard
    const dashboard = document.createElement('div');
    dashboard.className = 'dashboard';

    dashboard.innerHTML = this.buildHeaderHTML(weekTitle, focusText, stats);

    // Two column layout
    const grid = document.createElement('div');
    grid.className = 'dashboard-grid';

    const mainCol = document.createElement('div');
    mainCol.className = 'dashboard-main';

    const sideCol = document.createElement('div');
    sideCol.className = 'dashboard-sidebar';

    // Build sections
    if (sections.meetings) {
      mainCol.appendChild(this.buildMeetingsSection(sections.meetings, 0.3));
    }

    if (sections.actions) {
      mainCol.appendChild(this.buildActionsSection(sections.actions, 0.35));
    }

    if (sections.hygiene) {
      sideCol.appendChild(this.buildHygieneSection(sections.hygiene, 0.3));
    }

    if (sections.timeBlocks) {
      sideCol.appendChild(this.buildTimeBlocksSection(sections.timeBlocks, 0.35));
    }

    if (sections.focusAreas) {
      sideCol.appendChild(this.buildFocusSection(sections.focusAreas, 0.4));
    }

    grid.appendChild(mainCol);
    grid.appendChild(sideCol);
    dashboard.appendChild(grid);

    container.innerHTML = '';
    container.appendChild(dashboard);

    // Apply link enhancements via Enhancements module (consistent with other transforms)
    if (window.Enhancements) {
      Enhancements.links(container);
    }
  },

  /**
   * Find sections in the week overview
   * @param {Element} container - Container to search
   * @returns {Object} Map of section names to heading elements
   */
  findSections(container) {
    const sections = {};
    const h2s = container.querySelectorAll('h2');
    h2s.forEach(h2 => {
      const text = h2.textContent.toLowerCase();
      if (text.includes('meetings')) sections.meetings = h2;
      if (text.includes('action items')) sections.actions = h2;
      if (text.includes('hygiene')) sections.hygiene = h2;
      if (text.includes('impact')) sections.impact = h2;
      if (text.includes('time block') || text.includes('calendar block')) sections.timeBlocks = h2;
      if (text.includes('focus area')) sections.focusAreas = h2;
      if (text.includes('previous week')) sections.previousWeek = h2;
    });
    return sections;
  },

  /**
   * Count week overview stats
   * @param {Element} container - Container element
   * @param {Object} sections - Section headings map
   * @returns {Object} Stats object with counts
   */
  countStats(container, sections) {
    const stats = { meetings: 0, overdue: 0, dueThisWeek: 0, hygieneAlerts: 0 };

    if (sections.meetings) {
      const table = SectionUtils.findNextTable(sections.meetings);
      if (table) {
        stats.meetings = SectionUtils.getTableRows(table).length;
      }
    }

    if (sections.actions) {
      const content = SectionUtils.findContent(sections.actions);
      content.forEach(el => {
        if (el.tagName === 'H3') {
          const text = el.textContent;
          const textLower = text.toLowerCase();
          const countMatch = text.match(/\((\d+)\)/);
          if (countMatch) {
            const count = parseInt(countMatch[1]);
            if (textLower.includes('overdue')) stats.overdue += count;
            else if (textLower.includes('due this week')) stats.dueThisWeek += count;
          } else {
            let nextEl = el.nextElementSibling;
            while (nextEl && nextEl.tagName === 'UL') {
              const count = nextEl.querySelectorAll(':scope > li').length;
              if (textLower.includes('overdue')) stats.overdue += count;
              else if (textLower.includes('due this week')) stats.dueThisWeek += count;
              nextEl = nextEl.nextElementSibling;
            }
          }
        }
      });
    }

    if (sections.hygiene) {
      const content = SectionUtils.findContent(sections.hygiene);
      content.forEach(el => {
        if (el.tagName === 'TABLE') {
          stats.hygieneAlerts += SectionUtils.getTableRows(el).length;
        }
      });
    }

    return stats;
  },

  /**
   * Build header HTML with stats
   * @param {string} weekTitle - Week title
   * @param {string} focusText - Focus description
   * @param {Object} stats - Stats object
   * @returns {string} HTML string
   */
  buildHeaderHTML(weekTitle, focusText, stats) {
    return `
      <div class="dashboard-header">
        <div>
          <h1 class="dashboard-title">${weekTitle}</h1>
          <p class="dashboard-subtitle">${focusText}</p>
        </div>
      </div>
      <div class="stats-row">
        <div class="stat-card animate-in" style="animation-delay: 0.1s">
          <div class="stat-label">Customer Meetings</div>
          <div class="stat-value">${stats.meetings}</div>
          <div class="stat-meta">this week</div>
        </div>
        <div class="stat-card ${stats.overdue > 0 ? 'stat-card-warning' : ''} animate-in" style="animation-delay: 0.15s">
          <div class="stat-label">Overdue Actions</div>
          <div class="stat-value ${stats.overdue > 0 ? 'warning' : ''}">${stats.overdue}</div>
          <div class="stat-meta">need attention</div>
        </div>
        <div class="stat-card animate-in" style="animation-delay: 0.2s">
          <div class="stat-label">Due This Week</div>
          <div class="stat-value">${stats.dueThisWeek}</div>
          <div class="stat-meta">actions</div>
        </div>
        <div class="stat-card ${stats.hygieneAlerts > 0 ? 'stat-card-warning' : ''} animate-in" style="animation-delay: 0.25s">
          <div class="stat-label">Hygiene Alerts</div>
          <div class="stat-value ${stats.hygieneAlerts > 0 ? 'warning' : ''}">${stats.hygieneAlerts}</div>
          <div class="stat-meta">accounts</div>
        </div>
      </div>
    `;
  },

  /**
   * Build customer meetings section
   * @param {Element} heading - Meetings section heading
   * @param {number} delay - Animation delay
   * @returns {HTMLElement} Meetings section element
   */
  buildMeetingsSection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const table = SectionUtils.findNextTable(heading);
    let itemsHtml = '';

    if (table) {
      const rows = SectionUtils.getTableRows(table);
      let currentDay = '';

      rows.forEach((row, i) => {
        const cells = row.querySelectorAll('td');
        const day = cells[0]?.textContent.trim() || '';
        const time = cells[1]?.textContent.trim() || '';
        const account = cells[2]?.textContent.trim() || '';
        const ring = cells[3]?.textContent.trim() || '';
        const status = cells[4]?.textContent.trim() || '';

        const ringClass = SectionUtils.classifyRing(ring);
        // Check for various "needs work" indicators in prep status
        // Ready indicators: ✅ Prep ready, ✏️ Draft ready, ✅ Done
        // Needs work: 📋 Prep needed, 📅 Agenda needed, 🔄 Bring updates, 👥 Context needed
        const isReady = status.includes('✅') || status.includes('✏️') ||
                        status.toLowerCase().includes('ready') ||
                        status.toLowerCase().includes('done');
        const statusClass = isReady ? 'ready' : 'needs-prep';

        if (day !== currentDay) {
          currentDay = day;
          itemsHtml += `<div class="week-day-header">${day}</div>`;
        }

        // Clean up status text for display (remove emojis, normalize)
        const displayStatus = status.replace(/[📋📅🔄👥✅✏️]/g, '').trim() || (isReady ? 'Ready' : 'Needs prep');

        itemsHtml += `
          <div class="week-meeting-item animate-in-fast" style="animation-delay: ${0.1 + i * 0.03}s">
            <div class="week-meeting-time">${time}</div>
            <div class="week-meeting-content">
              <div class="week-meeting-account">${account}</div>
              <div class="week-meeting-meta">
                <span class="ring-badge ${ringClass}">${ring}</span>
                <span class="prep-status ${statusClass}"><span class="prep-icon ${isReady ? 'ready' : 'warning'}"></span> ${displayStatus}</span>
              </div>
            </div>
          </div>
        `;
      });
    }

    const content = SectionUtils.findContent(heading);
    let noteHtml = '';
    content.forEach(el => {
      if (el.tagName === 'P' && el.textContent.includes('Note:')) {
        noteHtml = `<div class="section-note">${el.innerHTML}</div>`;
      }
    });

    section.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">Customer Meetings</h3>
        <a href="/today/week-meetings" class="section-card-link" data-route="/today/week-meetings">View All -></a>
      </div>
      <div class="section-card-body">
        ${itemsHtml || '<p class="text-muted text-center">No meetings this week</p>'}
        ${noteHtml}
      </div>
    `;

    return section;
  },

  /**
   * Build actions section
   * @param {Element} heading - Actions section heading
   * @param {number} delay - Animation delay
   * @returns {HTMLElement} Actions section element
   */
  buildActionsSection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const content = SectionUtils.findContent(heading);
    let itemsHtml = '';

    content.forEach(el => {
      if (el.tagName === 'H3') {
        const groupTitle = el.textContent;
        const isOverdue = groupTitle.toLowerCase().includes('overdue');
        itemsHtml += `<div class="action-group-header ${isOverdue ? 'overdue' : ''}">${groupTitle}</div>`;
      }

      if (el.tagName === 'UL') {
        const items = el.querySelectorAll(':scope > li');
        items.forEach((li) => {
          const text = li.innerHTML;
          const hasCheckbox = li.querySelector('input[type="checkbox"]');

          itemsHtml += `
            <div class="week-action-item">
              ${hasCheckbox ? '<input type="checkbox" class="week-action-checkbox" />' : '<span class="week-action-bullet">*</span>'}
              <div class="week-action-text">${text.replace(/<input[^>]*>/g, '')}</div>
            </div>
          `;
        });
      }

      if (el.tagName === 'P' && el.textContent.trim()) {
        itemsHtml += `<p class="week-action-note">${el.innerHTML}</p>`;
      }
    });

    section.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">Action Items</h3>
        <a href="/today/week-actions" class="section-card-link" data-route="/today/week-actions">View All -></a>
      </div>
      <div class="section-card-body">
        ${itemsHtml || '<p class="text-muted text-center">No actions</p>'}
      </div>
    `;

    return section;
  },

  /**
   * Build hygiene alerts section
   * @param {Element} heading - Hygiene section heading
   * @param {number} delay - Animation delay
   * @returns {HTMLElement} Hygiene section element
   */
  buildHygieneSection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const content = SectionUtils.findContent(heading);
    let itemsHtml = '';
    let currentLevel = 'info';

    content.forEach(el => {
      if (el.tagName === 'H3') {
        const text = el.textContent;
        if (text.includes('\uD83D\uDD34') || text.toLowerCase().includes('critical')) currentLevel = 'critical';
        else if (text.includes('\uD83D\uDFE1') || text.toLowerCase().includes('attention')) currentLevel = 'warning';
        else if (text.includes('\uD83D\uDFE2') || text.toLowerCase().includes('healthy')) currentLevel = 'healthy';

        itemsHtml += `<div class="hygiene-level-header ${currentLevel}">${text}</div>`;
      }

      if (el.tagName === 'P' && (el.textContent.includes('None') || el.textContent.includes('\u2705') || el.textContent.includes('No '))) {
        itemsHtml += `<p class="text-muted text-sm">${el.textContent}</p>`;
      }

      if (el.tagName === 'TABLE') {
        const rows = SectionUtils.getTableRows(el);
        rows.forEach(row => {
          const cells = row.querySelectorAll('td');
          const account = cells[0]?.textContent.trim() || '';
          const ring = cells[1]?.textContent.trim() || '';
          const issue = cells[2]?.textContent.trim() || '';
          const action = cells[4]?.textContent.trim() || cells[3]?.textContent.trim() || '';

          itemsHtml += `
            <div class="hygiene-item ${currentLevel}">
              <div class="hygiene-item-account">${account}</div>
              <div class="hygiene-item-issue">${issue}</div>
              <div class="hygiene-item-action">${action}</div>
            </div>
          `;
        });
      }

      if (el.tagName === 'UL') {
        const items = el.querySelectorAll('li');
        items.forEach(li => {
          const text = li.textContent;
          const boldMatch = li.innerHTML.match(/<strong>([^<]+)<\/strong>\s*-?\s*(.*)/);
          if (boldMatch) {
            const account = boldMatch[1].trim();
            const issue = boldMatch[2].trim();
            itemsHtml += `
              <div class="hygiene-item ${currentLevel}">
                <div class="hygiene-item-account">${account}</div>
                <div class="hygiene-item-issue">${issue}</div>
              </div>
            `;
          } else {
            const icon = currentLevel === 'healthy' ? '\u2713' : (currentLevel === 'warning' ? '\u26A0' : '*');
            itemsHtml += `<div class="hygiene-${currentLevel}-item">${icon} ${text}</div>`;
          }
        });
      }
    });

    section.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: 8px;">
            <path d="M22 12h-4l-3 9L9 3l-3 9H2"></path>
          </svg>
          Account Hygiene
        </h3>
        <a href="/today/week-hygiene" class="section-card-link" data-route="/today/week-hygiene">View All -></a>
      </div>
      <div class="section-card-body">
        ${itemsHtml || '<p class="text-muted text-center">All accounts healthy</p>'}
      </div>
    `;

    return section;
  },

  /**
   * Build time blocks section
   * @param {Element} heading - Time blocks section heading
   * @param {number} delay - Animation delay
   * @returns {HTMLElement} Time blocks section element
   */
  buildTimeBlocksSection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const content = SectionUtils.findContent(heading);
    let itemsHtml = '';

    content.forEach(el => {
      if (el.tagName === 'TABLE') {
        const rows = SectionUtils.getTableRows(el);
        rows.forEach(row => {
          const cells = row.querySelectorAll('td');
          const block = cells[0]?.textContent.trim() || '';
          const day = cells[1]?.textContent.trim() || '';
          const duration = cells[2]?.textContent.trim() || '';

          itemsHtml += `
            <div class="time-block-item">
              <div class="time-block-day-name">${day}</div>
              <div class="time-block-task">${block}</div>
              <div class="time-block-duration">${duration}</div>
            </div>
          `;
        });
      }

      if (el.tagName === 'P' && el.querySelector('strong')) {
        const dayName = el.querySelector('strong').textContent;
        const details = el.textContent.replace(dayName, '').trim();
        itemsHtml += `
          <div class="time-block-day">
            <div class="time-block-day-name">${dayName}</div>
            <div class="time-block-day-info">${details}</div>
          </div>
        `;
      }

      if (el.tagName === 'UL') {
        const items = el.querySelectorAll('li');
        items.forEach(li => {
          itemsHtml += `<div class="time-block-item">${li.innerHTML}</div>`;
        });
      }
    });

    section.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: 8px;">
            <rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
            <line x1="16" y1="2" x2="16" y2="6"></line>
            <line x1="8" y1="2" x2="8" y2="6"></line>
            <line x1="3" y1="10" x2="21" y2="10"></line>
          </svg>
          Calendar Blocks
        </h3>
      </div>
      <div class="section-card-body">
        ${itemsHtml || '<p class="text-muted text-center">No time blocks defined</p>'}
      </div>
    `;

    return section;
  },

  /**
   * Build focus areas section
   * @param {Element} heading - Focus areas section heading
   * @param {number} delay - Animation delay
   * @returns {HTMLElement} Focus areas section element
   */
  buildFocusSection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const content = SectionUtils.findContent(heading);
    let itemsHtml = '';

    content.forEach(el => {
      if (el.tagName === 'OL') {
        const items = el.querySelectorAll('li');
        items.forEach((li, i) => {
          itemsHtml += `
            <div class="focus-area-item">
              <span class="focus-area-number">${i + 1}</span>
              <div class="focus-area-text">${li.innerHTML}</div>
            </div>
          `;
        });
      }
    });

    section.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: 8px;">
            <circle cx="12" cy="12" r="10"></circle>
            <circle cx="12" cy="12" r="6"></circle>
            <circle cx="12" cy="12" r="2"></circle>
          </svg>
          Focus Areas
        </h3>
        <a href="/today/week-focus" class="section-card-link" data-route="/today/week-focus">View All -></a>
      </div>
      <div class="section-card-body">
        ${itemsHtml || '<p class="text-muted text-center">No focus areas defined</p>'}
      </div>
    `;

    return section;
  }
};

// Register with TransformRegistry
if (window.TransformRegistry) {
  TransformRegistry.register(WeekOverviewTransform);
}

// Make available globally
window.WeekOverviewTransform = WeekOverviewTransform;
