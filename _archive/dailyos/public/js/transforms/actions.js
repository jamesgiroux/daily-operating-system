/**
 * DailyOS Actions Page Transform
 * Transforms the actions page into a card-based layout
 *
 * @module transforms/actions
 */

const ActionsTransform = {
  /**
   * Transform name for registry identification
   * @type {string}
   */
  name: 'actions',

  /**
   * Detect if this is an actions page
   * @param {Element} container - DOM container to check
   * @returns {boolean} True if this is an actions page
   */
  detect(container) {
    const h1 = container.querySelector('h1');
    return h1 && h1.textContent.includes('Action Items');
  },

  /**
   * Apply the actions page transformation
   * @param {Element} container - DOM container to transform
   */
  apply(container) {
    const h1 = container.querySelector('h1');
    const dateMatch = h1.textContent.match(/- (.+)/);
    const dateStr = dateMatch ? dateMatch[1] : 'Actions';

    // Find all h2 sections
    const sections = this.findSections(container);

    // Count items for stats
    const stats = this.countItems(container, sections);

    // Build dashboard
    const dashboard = document.createElement('div');
    dashboard.className = 'dashboard';

    dashboard.innerHTML = this.buildHeaderHTML(dateStr, stats);

    // Create main content area
    const mainContent = document.createElement('div');
    mainContent.className = 'actions-layout';

    // Build sections
    if (sections.overdue && stats.overdue > 0) {
      mainContent.appendChild(this.buildActionSection(sections.overdue, 'Overdue', 'warning', 0.3));
    }

    if (sections.dueToday) {
      mainContent.appendChild(this.buildActionSection(sections.dueToday, 'Due Today', 'today', 0.35));
    }

    if (sections.dueThisWeek) {
      mainContent.appendChild(this.buildActionSection(sections.dueThisWeek, 'Due This Week', 'week', 0.4));
    }

    if (sections.dueLater) {
      mainContent.appendChild(this.buildActionSection(sections.dueLater, 'Due Later', 'later', 0.45));
    }

    if (sections.waiting) {
      mainContent.appendChild(this.buildWaitingSection(sections.waiting, 0.5));
    }

    dashboard.appendChild(mainContent);
    container.innerHTML = '';
    container.appendChild(dashboard);
  },

  /**
   * Find sections in the actions page
   * @param {Element} container - Container to search
   * @returns {Object} Map of section names to heading elements
   */
  findSections(container) {
    const sections = {};
    const h2s = container.querySelectorAll('h2');
    h2s.forEach(h2 => {
      const text = h2.textContent.toLowerCase();
      if (text.includes('overdue')) sections.overdue = h2;
      if (text.includes('due today')) sections.dueToday = h2;
      if (text.includes('due this week')) sections.dueThisWeek = h2;
      if (text.includes('due later')) sections.dueLater = h2;
      if (text.includes('waiting')) sections.waiting = h2;
      if (text.includes('related')) sections.related = h2;
      if (text.includes('other')) sections.others = h2;
    });
    return sections;
  },

  /**
   * Count action items in each section
   * @param {Element} container - Container element
   * @param {Object} sections - Section headings map
   * @returns {Object} Stats object with counts
   */
  countItems(container, sections) {
    const stats = { overdue: 0, dueToday: 0, dueThisWeek: 0, waiting: 0 };

    if (sections.overdue) {
      const content = SectionUtils.findContent(sections.overdue);
      content.forEach(el => {
        if (el.tagName === 'UL') {
          stats.overdue += el.querySelectorAll(':scope > li').length;
        }
      });
      if (stats.overdue === 0) {
        const text = content.map(el => el.textContent).join(' ').toLowerCase();
        if (text.includes('no overdue')) stats.overdue = 0;
      }
    }

    if (sections.dueToday) {
      const content = SectionUtils.findContent(sections.dueToday);
      content.forEach(el => {
        if (el.tagName === 'UL') {
          stats.dueToday += el.querySelectorAll(':scope > li').length;
        }
      });
    }

    if (sections.dueThisWeek) {
      const content = SectionUtils.findContent(sections.dueThisWeek);
      content.forEach(el => {
        if (el.tagName === 'UL') {
          const items = el.querySelectorAll(':scope > li');
          items.forEach(li => {
            if (li.querySelector('input[type="checkbox"]') || li.textContent.includes('[ ]')) {
              stats.dueThisWeek++;
            }
          });
        }
      });
    }

    if (sections.waiting) {
      const table = SectionUtils.findNextTable(sections.waiting);
      if (table) {
        stats.waiting = SectionUtils.getTableRows(table).length;
      }
    }

    return stats;
  },

  /**
   * Build header HTML with stats
   * @param {string} dateStr - Date string
   * @param {Object} stats - Stats object
   * @returns {string} HTML string
   */
  buildHeaderHTML(dateStr, stats) {
    return `
      <div class="dashboard-header">
        <div>
          <h1 class="dashboard-title">Action Items</h1>
          <p class="dashboard-subtitle">${dateStr}</p>
        </div>
      </div>
      <div class="stats-row">
        <div class="stat-card ${stats.overdue > 0 ? 'stat-card-warning' : ''} animate-in" style="animation-delay: 0.1s">
          <div class="stat-label">Overdue</div>
          <div class="stat-value ${stats.overdue > 0 ? 'warning' : ''}">${stats.overdue}</div>
          <div class="stat-meta">need attention</div>
        </div>
        <div class="stat-card animate-in" style="animation-delay: 0.15s">
          <div class="stat-label">Due Today</div>
          <div class="stat-value">${stats.dueToday}</div>
          <div class="stat-meta">items</div>
        </div>
        <div class="stat-card animate-in" style="animation-delay: 0.2s">
          <div class="stat-label">Due This Week</div>
          <div class="stat-value">${stats.dueThisWeek}</div>
          <div class="stat-meta">items</div>
        </div>
        <div class="stat-card animate-in" style="animation-delay: 0.25s">
          <div class="stat-label">Waiting On</div>
          <div class="stat-value">${stats.waiting}</div>
          <div class="stat-meta">delegated</div>
        </div>
      </div>
    `;
  },

  /**
   * Build an action section card
   * @param {Element} heading - Section heading
   * @param {string} title - Section title
   * @param {string} type - Section type for styling
   * @param {number} delay - Animation delay
   * @returns {HTMLElement} Section card element
   */
  buildActionSection(heading, title, type, delay) {
    const section = document.createElement('div');
    section.className = `section-card section-card-${type} animate-in`;
    section.style.animationDelay = `${delay}s`;

    const content = SectionUtils.findContent(heading);
    let itemsHtml = '';
    let hasItems = false;

    content.forEach(el => {
      if (el.tagName === 'UL') {
        const items = el.querySelectorAll(':scope > li');
        items.forEach((li, i) => {
          const itemHtml = this.parseActionItem(li, i);
          if (itemHtml) {
            hasItems = true;
            itemsHtml += itemHtml;
          }
        });
      }
      if (el.tagName === 'H3') {
        const groupTitle = el.textContent;
        let nextEl = el.nextElementSibling;
        if (nextEl && nextEl.tagName === 'UL') {
          const items = nextEl.querySelectorAll(':scope > li');
          if (items.length > 0) {
            itemsHtml += `<div class="action-group-header">${groupTitle}</div>`;
            items.forEach((li, i) => {
              const itemHtml = this.parseActionItem(li, i);
              if (itemHtml) {
                hasItems = true;
                itemsHtml += itemHtml;
              }
            });
          }
        }
      }
      if (el.tagName === 'P' && (el.textContent.includes('No ') || el.textContent.includes('*No '))) {
        itemsHtml = `<p class="text-muted text-center">${el.textContent.replace(/\*/g, '')}</p>`;
      }
    });

    section.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">${title}</h3>
      </div>
      <div class="section-card-body">
        ${itemsHtml || '<p class="text-muted text-center">No items</p>'}
      </div>
    `;

    return section;
  },

  /**
   * Parse a single action item LI into card HTML
   *
   * Expects canonical format from markdown_primitives.py:
   * - [ ] **Title** - Account - Due: YYYY-MM-DD (X days overdue)
   *   - **Context**: Why this task exists
   *   - **Source**: Where it came from
   *   - **Owner**: Who is responsible
   *   - **Priority**: P1/P2/P3
   *
   * @param {HTMLLIElement} li - List item element
   * @param {number} index - Item index for animation
   * @returns {string|null} HTML string or null if not an action item
   */
  parseActionItem(li, index) {
    const text = li.innerHTML;
    const plainText = li.textContent;

    // Skip if not an action item (no checkbox or bold title)
    if (!text.includes('checkbox') && !text.includes('[ ]') && !li.querySelector('strong')) {
      if (li.parentElement.parentElement.tagName === 'LI') {
        return null;
      }
    }

    // Extract title from **bold** text
    const titleMatch = text.match(/<strong>([^<]+)<\/strong>/);
    const title = titleMatch ? titleMatch[1] : li.firstChild?.textContent?.trim() || 'Untitled';

    let account = '';
    let due = '';
    let overdue = '';
    let context = '';
    let source = '';
    let owner = '';
    let priority = '';

    // Parse main line: **Title** - Account - Due: YYYY-MM-DD (X days overdue)
    const mainLine = plainText.split('\n')[0];
    const titleEnd = titleMatch ? mainLine.indexOf(titleMatch[1]) + titleMatch[1].length : 0;
    const dueStart = mainLine.indexOf(' - Due:');

    // Extract account (text between title and Due:)
    if (dueStart > titleEnd) {
      let accountText = mainLine.substring(titleEnd, dueStart).trim();
      if (accountText.startsWith(' - ')) accountText = accountText.substring(3);
      if (accountText.startsWith('- ')) accountText = accountText.substring(2);
      account = accountText.trim();
    }

    // Extract due date
    const dueMatch = mainLine.match(/Due:\s*(\d{4}-\d{2}-\d{2})/);
    if (dueMatch) {
      due = dueMatch[1];
    }

    // Extract overdue indicator
    const overdueMatch = mainLine.match(/\((\d+)\s*days?\s*overdue\)/i);
    if (overdueMatch) {
      overdue = overdueMatch[1] + ' days overdue';
      priority = 'Overdue';
    }

    // Parse sub-bullets with canonical **Label**: Value format
    const subList = li.querySelector('ul');
    if (subList) {
      subList.querySelectorAll('li').forEach(sub => {
        // Match canonical format: <strong>Label</strong>: Value
        const labelMatch = sub.innerHTML.match(/<strong>(\w+)<\/strong>:\s*(.+)/);
        if (labelMatch) {
          const label = labelMatch[1].toLowerCase();
          const value = labelMatch[2].replace(/<[^>]+>/g, '').trim(); // Strip HTML tags

          switch (label) {
            case 'context': context = value; break;
            case 'source': source = value; break;
            case 'owner': owner = value; break;
            case 'priority': priority = value; break;
            case 'account': account = account || value; break;
          }
        }
      });
    }

    // Determine priority display
    let priorityClass = 'p2';
    let priorityText = 'P2';
    if (overdue) {
      priorityClass = 'overdue';
      priorityText = overdue;
    } else if (priority) {
      priorityClass = priority.toLowerCase().replace(/\s/g, '');
      priorityText = priority;
    }

    // Build description from context and source
    let description = '';
    if (context) {
      description = context;
    }
    if (source && !description.includes(source)) {
      description += description ? ` <span class="action-item-source">(${source})</span>` : source;
    }

    return `
      <div class="action-item animate-in-fast" style="animation-delay: ${0.1 + index * 0.05}s">
        <div class="action-item-checkbox">
          <input type="checkbox" />
        </div>
        <div class="action-item-content">
          <div class="action-item-header">
            <span class="action-item-title">${title}</span>
            ${priorityText ? `<span class="priority-badge ${priorityClass}">${priorityText}</span>` : ''}
          </div>
          <div class="action-item-meta">
            ${account ? `<span class="action-item-account">${account}</span>` : ''}
            ${due && !overdue ? `<span class="action-item-due">Due: ${due}</span>` : ''}
            ${owner && owner.toLowerCase() !== 'james' ? `<span class="action-item-owner">Owner: ${owner}</span>` : ''}
          </div>
          ${description ? `<div class="action-item-context">${description}</div>` : ''}
        </div>
      </div>
    `;
  },

  /**
   * Build waiting-on section card
   * @param {Element} heading - Waiting section heading
   * @param {number} delay - Animation delay
   * @returns {HTMLElement} Waiting section element
   */
  buildWaitingSection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const table = SectionUtils.findNextTable(heading);
    let itemsHtml = '';

    if (table) {
      const rows = SectionUtils.getTableRows(table);
      rows.forEach((row, i) => {
        const cells = row.querySelectorAll('td');
        const who = cells[0]?.textContent.trim() || '';
        const what = cells[1]?.textContent.trim() || '';
        const asked = cells[2]?.textContent.trim() || '';
        const days = cells[3]?.textContent.trim() || '';
        const context = cells[4]?.textContent.trim() || '';

        itemsHtml += `
          <div class="waiting-item animate-in-fast" style="animation-delay: ${0.1 + i * 0.05}s">
            <div class="waiting-item-who">${who}</div>
            <div class="waiting-item-content">
              <div class="waiting-item-what">${what}</div>
              ${context ? `<div class="waiting-item-context">${context}</div>` : ''}
            </div>
            <div class="waiting-item-days">${days} days</div>
          </div>
        `;
      });
    }

    section.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: 8px;">
            <circle cx="12" cy="12" r="10"></circle>
            <polyline points="12 6 12 12 16 14"></polyline>
          </svg>
          Waiting On
        </h3>
      </div>
      <div class="section-card-body">
        ${itemsHtml || '<p class="text-muted text-center">Nothing pending</p>'}
      </div>
    `;

    return section;
  }
};

// Register with TransformRegistry
if (window.TransformRegistry) {
  TransformRegistry.register(ActionsTransform);
}

// Make available globally
window.ActionsTransform = ActionsTransform;
