/**
 * DailyOS Email Page Transform
 * Transforms the email summary page into a card-based layout
 *
 * @module transforms/email
 */

/**
 * Escape HTML special characters to prevent XSS and broken markup
 * @param {string} str - String to escape
 * @returns {string} Escaped string safe for HTML insertion
 */
function escapeHtml(str) {
  if (!str) return '';
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

const EmailTransform = {
  /**
   * Transform name for registry identification
   * @type {string}
   */
  name: 'email',

  /**
   * Detect if this is an email summary page
   * @param {Element} container - DOM container to check
   * @returns {boolean} True if this is an email page
   */
  detect(container) {
    const h1 = container.querySelector('h1');
    return h1 && h1.textContent.includes('Email Summary');
  },

  /**
   * Apply the email page transformation
   * @param {Element} container - DOM container to transform
   */
  apply(container) {
    const h1 = container.querySelector('h1');
    const dateMatch = h1.textContent.match(/- (.+)/);
    const dateStr = dateMatch ? dateMatch[1] : 'Email Summary';

    // Find sections
    const sections = this.findSections(container);

    // Count emails
    const { highCount, mediumCount, archivedCount } = this.countEmails(sections);

    // Build dashboard
    const dashboard = document.createElement('div');
    dashboard.className = 'dashboard';

    dashboard.innerHTML = this.buildHeaderHTML(dateStr, highCount, mediumCount, archivedCount);

    // Build email sections
    const mainContent = document.createElement('div');
    mainContent.className = 'email-layout';

    if (sections.high) {
      mainContent.appendChild(this.buildEmailSection(sections.high, 'High Priority', 'high', 0.3));
    }

    if (sections.medium) {
      mainContent.appendChild(this.buildEmailSection(sections.medium, 'Medium Priority', 'medium', 0.35));
    }

    dashboard.appendChild(mainContent);
    container.innerHTML = '';
    container.appendChild(dashboard);
  },

  /**
   * Find sections in the email page
   * @param {Element} container - Container to search
   * @returns {Object} Map of section names to heading elements
   */
  findSections(container) {
    const sections = {};
    const h2s = container.querySelectorAll('h2');
    h2s.forEach(h2 => {
      const text = h2.textContent.toLowerCase();
      if (text.includes('high priority')) sections.high = h2;
      if (text.includes('medium priority')) sections.medium = h2;
      if (text.includes('calendar')) sections.calendar = h2;
      if (text.includes('archive')) sections.archived = h2;
      if (text.includes('summary')) sections.summary = h2;
    });
    return sections;
  },

  /**
   * Count emails in each priority section
   * @param {Object} sections - Section headings map
   * @returns {Object} Count object with highCount, mediumCount, archivedCount
   */
  countEmails(sections) {
    let highCount = 0, mediumCount = 0, archivedCount = 0;

    // Extract high priority count
    if (sections.high) {
      const headingMatch = sections.high.textContent.match(/\((\d+)\)/);
      if (headingMatch) {
        highCount = parseInt(headingMatch[1]);
      } else {
        const content = SectionUtils.findContent(sections.high);
        content.forEach(el => {
          if (el.tagName === 'H3') highCount++;
        });
        const table = SectionUtils.findNextTable(sections.high);
        if (table && highCount === 0) {
          highCount = SectionUtils.getTableRows(table).length;
        }
      }
    }

    // Extract medium priority count
    if (sections.medium) {
      const headingMatch = sections.medium.textContent.match(/\((\d+)\)/);
      if (headingMatch) {
        mediumCount = parseInt(headingMatch[1]);
      } else {
        const table = SectionUtils.findNextTable(sections.medium);
        if (table) {
          mediumCount = SectionUtils.getTableRows(table).length;
        }
      }
    }

    // Extract archived count if present
    if (sections.archived) {
      const headingMatch = sections.archived.textContent.match(/\((\d+)\)/);
      if (headingMatch) {
        archivedCount = parseInt(headingMatch[1]);
      }
    }

    return { highCount, mediumCount, archivedCount };
  },

  /**
   * Build header HTML with stats
   * @param {string} dateStr - Date string
   * @param {number} highCount - High priority count
   * @param {number} mediumCount - Medium priority count
   * @param {number} archivedCount - Archived count
   * @returns {string} HTML string
   */
  buildHeaderHTML(dateStr, highCount, mediumCount, archivedCount) {
    return `
      <div class="dashboard-header">
        <div>
          <h1 class="dashboard-title">Email Summary</h1>
          <p class="dashboard-subtitle">${dateStr}</p>
        </div>
      </div>
      <div class="stats-row">
        <div class="stat-card ${highCount > 0 ? 'stat-card-warning' : ''} animate-in" style="animation-delay: 0.1s">
          <div class="stat-label">High Priority</div>
          <div class="stat-value ${highCount > 0 ? 'warning' : ''}">${highCount}</div>
          <div class="stat-meta">need attention</div>
        </div>
        <div class="stat-card animate-in" style="animation-delay: 0.15s">
          <div class="stat-label">Medium Priority</div>
          <div class="stat-value">${mediumCount}</div>
          <div class="stat-meta">for review</div>
        </div>
        <div class="stat-card animate-in" style="animation-delay: 0.2s">
          <div class="stat-label">Archived</div>
          <div class="stat-value">${archivedCount}</div>
          <div class="stat-meta">processed</div>
        </div>
      </div>
    `;
  },

  /**
   * Build email section card
   * @param {Element} heading - Section heading
   * @param {string} title - Section title
   * @param {string} type - Section type for styling
   * @param {number} delay - Animation delay
   * @returns {HTMLElement} Section card element
   */
  buildEmailSection(heading, title, type, delay) {
    const section = document.createElement('div');
    section.className = `section-card section-card-${type === 'high' ? 'warning' : 'week'} animate-in`;
    section.style.animationDelay = `${delay}s`;

    const content = SectionUtils.findContent(heading, true);
    let itemsHtml = '';
    let emailIndex = 0;

    for (let i = 0; i < content.length; i++) {
      const el = content[i];

      if (el.tagName === 'HR') {
        continue;
      }

      if (el.tagName === 'H3') {
        const h3Text = el.textContent.trim();
        const subjectMatch = h3Text.match(/^\d+\.\s*(.+)$/);

        if (subjectMatch) {
          const subject = subjectMatch[1];
          let from = '';
          let snippet = '';
          let classification = '';

          for (let j = i + 1; j < content.length; j++) {
            const nextEl = content[j];
            if (!nextEl || !nextEl.tagName) continue;

            if (nextEl.tagName === 'H3' || nextEl.tagName === 'H2' || nextEl.tagName === 'HR') {
              break;
            }

            if (nextEl.tagName === 'P') {
              const text = nextEl.textContent;
              const html = nextEl.innerHTML;

              if (html.includes('<strong>From</strong>') || text.match(/^\*\*From\*\*/)) {
                const fromMatch = text.match(/From[:\s]*([^]*?)(?=Date|$)/i);
                if (fromMatch) {
                  from = fromMatch[1].trim().replace(/\*\*/g, '').trim();
                }
              }

              if (text.includes('Classification')) {
                classification = text.replace(/Classification[:\s]*/i, '').trim();
              }

              if (text.includes('Snippet') && !text.includes('>')) {
                continue;
              }
            }

            if (nextEl.tagName === 'BLOCKQUOTE') {
              snippet = nextEl.textContent.trim();
              if (snippet.length > 150) snippet = snippet.substring(0, 150) + '...';
            }
          }

          let typeClass = 'info';
          if (classification.includes('ACTION')) typeClass = 'action';
          else if (classification.includes('OPPORTUNITY')) typeClass = 'opportunity';
          else if (classification.includes('RISK')) typeClass = 'risk';

          itemsHtml += `
            <div class="email-list-item animate-in-fast" style="animation-delay: ${0.1 + emailIndex * 0.05}s">
              <div class="email-list-badge ${typeClass}"></div>
              <div class="email-list-content">
                <div class="email-list-from">${escapeHtml(from)}</div>
                <div class="email-list-subject">${escapeHtml(subject)}</div>
                ${snippet ? `<div class="email-list-snippet">${escapeHtml(snippet)}</div>` : ''}
              </div>
            </div>
          `;
          emailIndex++;
        } else {
          itemsHtml += `<div class="email-group-header">${escapeHtml(h3Text)}</div>`;
        }
      }

      if (el.tagName === 'TABLE') {
        const rows = SectionUtils.getTableRows(el);
        rows.forEach((row, idx) => {
          const cells = row.querySelectorAll('td');
          const from = cells[0]?.textContent.trim() || '';
          const subject = cells[1]?.textContent.trim() || '';
          const emailType = cells[2]?.textContent.trim() || '';
          const action = cells[3]?.textContent.trim() || '';

          let typeClass = 'info';
          const typeLower = emailType.toLowerCase();
          if (typeLower.includes('action')) typeClass = 'action';
          else if (typeLower.includes('opportunity')) typeClass = 'opportunity';

          itemsHtml += `
            <div class="email-list-item animate-in-fast" style="animation-delay: ${0.1 + emailIndex * 0.05}s">
              <div class="email-list-badge ${typeClass}"></div>
              <div class="email-list-content">
                <div class="email-list-from">${escapeHtml(from)}</div>
                <div class="email-list-subject">${escapeHtml(subject)}</div>
                ${action ? `<div class="email-list-action">${escapeHtml(action)}</div>` : ''}
              </div>
              <div class="email-list-type">${escapeHtml(emailType)}</div>
            </div>
          `;
          emailIndex++;
        });
      }
    }

    section.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">${title}</h3>
      </div>
      <div class="section-card-body">
        ${itemsHtml || '<p class="text-muted text-center">No emails</p>'}
      </div>
    `;

    return section;
  }
};

// Register with TransformRegistry
if (window.TransformRegistry) {
  TransformRegistry.register(EmailTransform);
}

// Make available globally
window.EmailTransform = EmailTransform;
