/**
 * DailyOS Overview Page Transform
 * Transforms the daily overview page into a dashboard layout
 *
 * @module transforms/overview
 */

const OverviewTransform = {
  /**
   * Transform name for registry identification
   * @type {string}
   */
  name: 'overview',

  /**
   * Detect if this is an overview page
   * @param {Element} container - DOM container to check
   * @returns {boolean} True if this is an overview page
   */
  detect(container) {
    const h1 = container.querySelector('h1');
    return h1 && h1.textContent.includes('Today:');
  },

  /**
   * Apply the overview page transformation
   * @param {Element} container - DOM container to transform
   */
  apply(container) {
    const h1 = container.querySelector('h1');
    if (!h1) return;

    // Parse the date from title
    const dateMatch = h1.textContent.match(/Today:\s*(.+)/);
    const dateStr = dateMatch ? dateMatch[1] : 'Today';

    // Find key sections
    const sections = this.findSections(container);

    // Build dashboard structure
    const dashboard = document.createElement('div');
    dashboard.className = 'dashboard';

    // Dashboard header with stats
    const stats = this.extractStats(container, sections);
    dashboard.innerHTML = this.buildHeaderHTML(dateStr, stats);

    // Create two-column grid
    const grid = document.createElement('div');
    grid.className = 'dashboard-grid';

    const mainCol = document.createElement('div');
    mainCol.className = 'dashboard-main';

    const sideCol = document.createElement('div');
    sideCol.className = 'dashboard-sidebar';

    // Transform schedule into timeline
    if (sections.schedule) {
      mainCol.appendChild(this.buildScheduleTimeline(sections.schedule));
    }

    // Transform customer meetings into cards
    if (sections.customerMeetings) {
      mainCol.appendChild(this.buildCustomerMeetingsSection(sections.customerMeetings));
    }

    // Agenda status section (upcoming meetings needing agendas)
    if (sections.agenda) {
      mainCol.appendChild(this.buildAgendaStatusSection(sections.agenda));
    }

    // Build sidebar cards
    if (sections.email) {
      sideCol.appendChild(this.buildEmailSummaryCard(sections.email));
    }

    if (sections.actions) {
      sideCol.appendChild(this.buildActionsCard(sections.actions));
    }

    if (sections.waiting) {
      sideCol.appendChild(this.buildWaitingCard(sections.waiting));
    }

    if (sections.focus) {
      sideCol.appendChild(this.buildFocusCard(sections.focus));
    }

    grid.appendChild(mainCol);
    grid.appendChild(sideCol);
    dashboard.appendChild(grid);

    // Replace entire content
    container.innerHTML = '';
    container.appendChild(dashboard);

    // Add click handlers for prep doc links
    this.addPrepDocClickHandlers(container);
  },

  /**
   * Find key sections in the overview content
   * @param {Element} container - Container to search
   * @returns {Object} Map of section names to heading elements
   */
  findSections(container) {
    const sections = {};
    const h2s = container.querySelectorAll('h2');
    h2s.forEach(h2 => {
      const text = h2.textContent.toLowerCase();
      if (text.includes('schedule')) sections.schedule = h2;
      if (text.includes('customer meetings')) sections.customerMeetings = h2;
      if (text.includes('email')) sections.email = h2;
      if (text.includes('action items')) sections.actions = h2;
      if (text.includes('waiting')) sections.waiting = h2;
      if (text.includes('suggested focus')) sections.focus = h2;
      if (text.includes('agenda status')) sections.agenda = h2;
      if (text.includes('files')) sections.files = h2;
    });
    return sections;
  },

  /**
   * Extract stats from overview content
   * @param {Element} container - Container element
   * @param {Object} sections - Section headings map
   * @returns {Object} Stats object with counts
   */
  extractStats(container, sections) {
    const stats = {
      totalMeetings: 0,
      customerMeetings: 0,
      actionsDue: 0,
      highPriorityEmails: 0,
      waitingOn: 0
    };

    // Count schedule rows
    if (sections.schedule) {
      const table = SectionUtils.findNextTable(sections.schedule);
      if (table) {
        const rows = SectionUtils.getTableRows(table);
        stats.totalMeetings = rows.length;
        rows.forEach(row => {
          const typeCell = row.querySelector('td:nth-child(3)');
          if (typeCell && typeCell.textContent.toLowerCase().includes('customer')) {
            stats.customerMeetings++;
          }
        });
      }
    }

    // Count actions - prefer explicit counts in headers, fall back to counting items
    if (sections.actions) {
      const content = SectionUtils.findContent(sections.actions);
      let headerCount = 0;
      let itemCount = 0;

      content.forEach(el => {
        if (el.tagName === 'H3') {
          // Extract count from header like "Overdue (10)"
          const match = el.textContent.match(/\((\d+)\)/);
          if (match) {
            headerCount += parseInt(match[1]);
          }
        }
        if (el.tagName === 'UL') {
          // Count actual checkbox items
          const checkboxItems = el.querySelectorAll('li input[type="checkbox"]');
          if (checkboxItems.length > 0) {
            itemCount += checkboxItems.length;
          } else {
            // Count regular list items if no checkboxes
            itemCount += el.querySelectorAll(':scope > li').length;
          }
        }
        if (el.tagName === 'TABLE') {
          const rows = SectionUtils.getTableRows(el);
          itemCount += rows.length;
        }
      });

      // Use header count if available (more accurate), otherwise use item count
      stats.actionsDue = headerCount > 0 ? headerCount : itemCount;
    }

    // Count high priority emails
    if (sections.email) {
      let match = sections.email.textContent.match(/\((\d+)\)/);
      if (!match) {
        const content = SectionUtils.findContent(sections.email);
        for (const el of content) {
          if (el.tagName === 'H3') {
            match = el.textContent.match(/\((\d+)\)/);
            if (match) break;
          }
        }
      }
      if (match) {
        stats.highPriorityEmails = parseInt(match[1]);
      }
    }

    // Count waiting items
    if (sections.waiting) {
      const table = SectionUtils.findNextTable(sections.waiting);
      if (table) {
        stats.waitingOn = SectionUtils.getTableRows(table).length;
      }
    }

    return stats;
  },

  /**
   * Build dashboard header HTML
   * @param {string} dateStr - Date string
   * @param {Object} stats - Stats object
   * @returns {string} HTML string
   */
  buildHeaderHTML(dateStr, stats) {
    return `
      <div class="dashboard-header">
        <div>
          <h1 class="dashboard-title">${dateStr}</h1>
          <p class="dashboard-subtitle">Daily operating dashboard</p>
        </div>
      </div>
      <div class="stats-row">
        <div class="stat-card animate-in" style="animation-delay: 0.1s">
          <div class="stat-label">Meetings</div>
          <div class="stat-value">${stats.totalMeetings}</div>
          <div class="stat-meta">${stats.customerMeetings} customer</div>
        </div>
        <div class="stat-card animate-in" style="animation-delay: 0.15s">
          <div class="stat-label">Actions Due</div>
          <div class="stat-value">${stats.actionsDue}</div>
          <div class="stat-meta">this week</div>
        </div>
        <div class="stat-card animate-in" style="animation-delay: 0.2s">
          <div class="stat-label">Emails</div>
          <div class="stat-value ${stats.highPriorityEmails > 0 ? 'customer' : ''}">${stats.highPriorityEmails}</div>
          <div class="stat-meta">high priority</div>
        </div>
        <div class="stat-card animate-in" style="animation-delay: 0.25s">
          <div class="stat-label">Waiting On</div>
          <div class="stat-value">${stats.waitingOn}</div>
          <div class="stat-meta">delegated</div>
        </div>
      </div>
    `;
  },

  /**
   * Build schedule timeline card
   * @param {Element} heading - Schedule section heading
   * @returns {HTMLElement} Schedule card element
   */
  buildScheduleTimeline(heading) {
    const card = document.createElement('div');
    card.className = 'section-card animate-in';
    card.style.animationDelay = '0.3s';

    const table = SectionUtils.findNextTable(heading);
    if (!table) {
      card.innerHTML = `
        <div class="section-card-header">
          <h3 class="section-card-title">Today's Schedule</h3>
        </div>
        <div class="section-card-body">
          <p class="text-muted">No schedule found</p>
        </div>
      `;
      return card;
    }

    const rows = SectionUtils.getTableRows(table);
    let timelineHtml = '';

    rows.forEach((row, i) => {
      const cells = row.querySelectorAll('td');
      const time = cells[0]?.textContent.trim() || '';
      const event = cells[1]?.textContent.trim() || '';
      const type = cells[2]?.textContent.trim().toLowerCase() || '';
      const prep = cells[3]?.textContent.trim() || '';
      const prepCell = cells[3];

      const isCustomer = type.includes('customer');
      const isInternal = type.includes('internal');
      const isProject = type.includes('project');

      const typeClass = isCustomer ? 'customer' : isInternal ? 'internal' : isProject ? 'project' : 'personal';
      const eventClean = event.replace(/\*\*/g, '');

      // Extract prep filename - check <code> element first, then text patterns
      let prepFileName = null;
      const codeEl = prepCell?.querySelector('code');
      if (codeEl && codeEl.textContent.includes('.md')) {
        prepFileName = codeEl.textContent.trim();
      } else {
        // Try backtick pattern in text
        const backtickMatch = prep.match(/`([^`]+\.md)`/);
        if (backtickMatch) {
          prepFileName = backtickMatch[1];
        } else {
          // Try to extract .md filename directly from text (with "See" prefix)
          const seeMatch = prep.match(/See\s+([^\s]+\.md)/i);
          if (seeMatch) {
            prepFileName = seeMatch[1];
          } else {
            // Try to extract .md filename directly
            const mdMatch = prep.match(/(\d{2}-\d{4}-[a-z]+-[a-z0-9-]+\.md)/);
            if (mdMatch) {
              prepFileName = mdMatch[1];
            }
          }
        }
      }

      // If no prep file found but we have a meeting type that typically has prep,
      // try to construct a matching filename pattern
      if (!prepFileName && (isCustomer || isProject || isInternal)) {
        // Convert time "4:00 PM" to "1600" format
        const timeMatch = time.match(/(\d{1,2}):(\d{2})\s*(AM|PM)/i);
        if (timeMatch) {
          let hours = parseInt(timeMatch[1]);
          const mins = timeMatch[2];
          const ampm = timeMatch[3].toUpperCase();
          if (ampm === 'PM' && hours !== 12) hours += 12;
          if (ampm === 'AM' && hours === 12) hours = 0;
          const timeCode = String(hours).padStart(2, '0') + mins;

          // Construct likely prep filename pattern
          const meetingType = isCustomer ? 'customer' : isProject ? 'project' : 'internal';
          // Pattern: NN-HHMM-type-*.md
          prepFileName = `*-${timeCode}-${meetingType}-*-prep.md`;
        }
      }

      // Has actual prep file if we extracted a filename
      const hasActualPrepFile = !!prepFileName && !prepFileName.includes('*');

      // Show prep status indicator (without link if no file)
      const showPrepStatus = prep && prep !== '-';

      // Build a searchable prep file pattern for linking
      let prepSearchPattern = null;
      if (!hasActualPrepFile && (isCustomer || isProject || isInternal)) {
        const timeMatch = time.match(/(\d{1,2}):(\d{2})\s*(AM|PM)/i);
        if (timeMatch) {
          let hours = parseInt(timeMatch[1]);
          const mins = timeMatch[2];
          const ampm = timeMatch[3].toUpperCase();
          if (ampm === 'PM' && hours !== 12) hours += 12;
          if (ampm === 'AM' && hours === 12) hours = 0;
          const timeCode = String(hours).padStart(2, '0') + mins;
          const meetingType = isCustomer ? 'customer' : isProject ? 'project' : 'internal';
          // Store pattern for potential file lookup
          prepSearchPattern = `${timeCode}-${meetingType}`;
        }
      }

      const hasPrepLink = hasActualPrepFile || prepSearchPattern;

      timelineHtml += `
        <div class="timeline-item animate-in-fast" style="animation-delay: ${0.35 + i * 0.05}s">
          <div class="timeline-marker">
            <div class="timeline-dot ${typeClass}"></div>
            <div class="timeline-time">${time}</div>
          </div>
          <div class="timeline-content ${typeClass}">
            <div class="timeline-title ${typeClass}">${eventClean}</div>
            <div class="timeline-meta">
              <span class="meeting-tag meeting-tag-${isCustomer ? 'customer' : isInternal ? 'internal' : isProject ? 'project' : 'personal'}">${type}</span>
              ${showPrepStatus ? `<span class="prep-status">${prep}</span>` : ''}
            </div>
            ${hasPrepLink ? `
              <div class="timeline-prep">
                <a href="#" class="timeline-prep-link" data-prep="${prepFileName || ''}" data-prep-search="${prepSearchPattern || ''}">
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                    <polyline points="14 2 14 8 20 8"></polyline>
                  </svg>
                  View Prep
                </a>
              </div>
            ` : ''}
          </div>
        </div>
      `;
    });

    card.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">
          Today's Schedule
          <span class="section-card-badge">${rows.length}</span>
        </h3>
      </div>
      <div class="section-card-body">
        <div class="timeline">
          ${timelineHtml}
        </div>
      </div>
    `;

    return card;
  },

  /**
   * Build customer meetings section card
   * @param {Element} heading - Customer meetings section heading
   * @returns {HTMLElement} Meetings card element
   */
  buildCustomerMeetingsSection(heading) {
    const card = document.createElement('div');
    card.className = 'section-card animate-in';
    card.style.animationDelay = '0.4s';

    const content = SectionUtils.findContent(heading);
    let cardsHtml = '';

    content.forEach(el => {
      if (el.tagName === 'H3') {
        const meetingName = el.textContent.replace(/\*\*/g, '');
        const details = {};

        let sibling = el.nextElementSibling;
        while (sibling && sibling.tagName === 'UL') {
          const items = sibling.querySelectorAll('li');
          items.forEach(li => {
            const text = li.textContent;
            if (text.includes('Ring:')) details.ring = text.split(':')[1]?.trim();
            if (text.includes('ARR:')) details.arr = text.split(':')[1]?.trim();
            if (text.includes('Renewal:')) details.renewal = text.split(':')[1]?.trim();
            if (text.includes('Context:')) details.context = text.split(':')[1]?.trim();
            if (text.includes('Focus:')) details.focus = text.split(':')[1]?.trim();
            if (text.includes('Prep:')) details.prep = text.match(/`([^`]+)`/)?.[1];
          });
          sibling = sibling.nextElementSibling;
        }

        const ringClass = SectionUtils.classifyRing(details.ring || '');

        cardsHtml += `
          <div class="meeting-card">
            <div class="meeting-card-header">
              <h4 class="meeting-card-title">${meetingName}</h4>
              <span class="ring-badge ${ringClass}">${details.ring || 'Unknown'}</span>
            </div>
            <div class="meeting-card-stats">
              ${details.arr ? `
                <div class="meeting-card-stat">
                  <span class="meeting-card-stat-label">ARR</span>
                  <span class="meeting-card-stat-value">${details.arr}</span>
                </div>
              ` : ''}
              ${details.renewal ? `
                <div class="meeting-card-stat">
                  <span class="meeting-card-stat-label">Renewal</span>
                  <span class="meeting-card-stat-value">${details.renewal}</span>
                </div>
              ` : ''}
            </div>
            ${details.context ? `<div class="meeting-card-context">${details.context}</div>` : ''}
            ${details.focus ? `<div class="meeting-card-context"><strong>Focus:</strong> ${details.focus}</div>` : ''}
            ${details.prep ? `
              <div class="meeting-card-actions">
                <a href="#" class="btn btn-secondary" data-prep="${details.prep}">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                    <polyline points="14 2 14 8 20 8"></polyline>
                  </svg>
                  Open Prep Doc
                </a>
              </div>
            ` : ''}
          </div>
        `;
      }
    });

    card.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">Customer Meetings</h3>
      </div>
      <div class="section-card-body" style="display: flex; flex-direction: column; gap: var(--space-4);">
        ${cardsHtml || '<p class="text-muted">No customer meetings today</p>'}
      </div>
    `;

    return card;
  },

  /**
   * Build agenda status section card
   * @param {Element} heading - Agenda status section heading
   * @returns {HTMLElement} Agenda status card element
   */
  buildAgendaStatusSection(heading) {
    const card = document.createElement('div');
    card.className = 'section-card animate-in';
    card.style.animationDelay = '0.45s';

    const table = SectionUtils.findNextTable(heading);
    let itemsHtml = '';

    if (table) {
      const rows = SectionUtils.getTableRows(table);
      rows.forEach((row, i) => {
        const cells = row.querySelectorAll('td');
        const meeting = cells[0]?.textContent.trim() || '';
        const date = cells[1]?.textContent.trim() || '';
        const status = cells[2]?.textContent.trim() || '';
        const action = cells[3]?.textContent.trim() || '';

        const needsAgenda = status.includes('Needs agenda') || status.includes('⚠️');
        const statusClass = needsAgenda ? 'needs-prep' : 'ready';
        const statusIcon = needsAgenda ? 'warning' : 'ready';

        // Extract draft file reference if present - check for backticks or direct path
        const actionCell = cells[3];
        let draftFile = null;
        const codeEl = actionCell?.querySelector('code');
        if (codeEl) {
          draftFile = codeEl.textContent.trim();
        } else {
          // Try backtick pattern
          const backtickMatch = action.match(/`([^`]+\.md)`/);
          if (backtickMatch) {
            draftFile = backtickMatch[1];
          } else {
            // Try direct path pattern
            const pathMatch = action.match(/90-agenda-needed\/[^\s]+\.md/);
            if (pathMatch) {
              draftFile = pathMatch[0];
            }
          }
        }

        itemsHtml += `
          <div class="meeting-card" style="animation-delay: ${0.1 + i * 0.05}s">
            <div class="meeting-card-header">
              <h4 class="meeting-card-title">${meeting}</h4>
              <span class="prep-status ${statusClass}">
                <span class="prep-icon ${statusIcon}"></span>
                ${status.replace(/[⚠️✅✏️]/g, '').trim()}
              </span>
            </div>
            <div class="meeting-card-stats">
              <div class="meeting-card-stat">
                <span class="meeting-card-stat-label">Date</span>
                <span class="meeting-card-stat-value">${date}</span>
              </div>
            </div>
            <div class="meeting-card-context">${action}</div>
            ${draftFile ? `
              <div class="meeting-card-actions">
                <a href="#" class="btn btn-secondary" data-prep="${draftFile}">
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"></path>
                    <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"></path>
                  </svg>
                  Review Draft
                </a>
              </div>
            ` : ''}
          </div>
        `;
      });
    }

    card.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="margin-right: 8px;">
            <rect x="3" y="4" width="18" height="18" rx="2" ry="2"></rect>
            <line x1="16" y1="2" x2="16" y2="6"></line>
            <line x1="8" y1="2" x2="8" y2="6"></line>
            <line x1="3" y1="10" x2="21" y2="10"></line>
          </svg>
          Agenda Status
        </h3>
        <span class="section-card-badge">${table ? SectionUtils.getTableRows(table).length : 0} upcoming</span>
      </div>
      <div class="section-card-body" style="display: flex; flex-direction: column; gap: var(--space-4);">
        ${itemsHtml || '<p class="text-muted">No agendas needed</p>'}
      </div>
    `;

    return card;
  },

  /**
   * Build email summary sidebar card
   * @param {Element} heading - Email section heading
   * @returns {HTMLElement} Email card element
   */
  buildEmailSummaryCard(heading) {
    const card = document.createElement('div');
    card.className = 'action-card animate-in';
    card.style.animationDelay = '0.35s';

    let count = 0;
    const countMatch = heading.textContent.match(/\((\d+)\)/);
    if (countMatch) {
      count = parseInt(countMatch[1]);
    } else {
      const content = SectionUtils.findContent(heading);
      for (const el of content) {
        if (el.tagName === 'H3') {
          const match = el.textContent.match(/\((\d+)\)/);
          if (match) {
            count = parseInt(match[1]);
            break;
          }
        }
      }
    }

    const content = SectionUtils.findContent(heading);
    let table = null;
    for (const el of content) {
      if (el.tagName === 'TABLE') {
        table = el;
        break;
      } else if (el.querySelector && el.querySelector('table')) {
        table = el.querySelector('table');
        break;
      }
    }

    let itemsHtml = '';
    const escapeHtml = (str) => str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');

    if (table) {
      const rows = SectionUtils.getTableRows(table);
      rows.forEach((row, i) => {
        if (i >= 4) return;
        const cells = row.querySelectorAll('td');
        const from = escapeHtml(cells[0]?.textContent.trim() || '');
        const subject = escapeHtml(cells[1]?.textContent.trim() || '');

        itemsHtml += `
          <div class="action-card-item">
            <div class="action-priority email">*</div>
            <div class="action-card-text">
              <div class="action-card-name">${from}</div>
              <div class="action-card-meta">${subject}</div>
            </div>
          </div>
        `;
      });
    }

    card.innerHTML = `
      <div class="action-card-header">
        <h3 class="action-card-title">Emails - Needs Attention</h3>
        <span class="action-card-count ${count > 0 ? 'warning' : ''}">${count} high priority</span>
      </div>
      <div class="action-card-body">
        ${itemsHtml || '<p class="text-muted">No high priority emails</p>'}
      </div>
      <div class="section-card-footer">
        <a href="/today/email" class="section-card-link">View all emails -></a>
      </div>
    `;

    return card;
  },

  /**
   * Build actions sidebar card
   * @param {Element} heading - Actions section heading
   * @returns {HTMLElement} Actions card element
   */
  buildActionsCard(heading) {
    const card = document.createElement('div');
    card.className = 'action-card animate-in';
    card.style.animationDelay = '0.4s';

    const content = SectionUtils.findContent(heading);
    let itemsHtml = '';
    let totalCount = 0;

    content.forEach(el => {
      if (el.tagName === 'TABLE') {
        const rows = SectionUtils.getTableRows(el);
        totalCount += rows.length;
        rows.forEach((row, i) => {
          if (i >= 5) return;
          const cells = row.querySelectorAll('td');
          const action = cells[0]?.textContent.trim() || '';
          const account = cells[1]?.textContent.trim() || '';
          const due = cells[2]?.textContent.trim() || '';
          const priority = cells[3]?.textContent.trim() || 'P2';

          const priorityClass = priority.toLowerCase().replace(' ', '');

          itemsHtml += `
            <div class="action-card-item">
              <div class="action-priority ${priorityClass}">${priority.charAt(0)}${priority.charAt(1) || ''}</div>
              <div class="action-card-text">
                <div class="action-card-name">${action}</div>
                <div class="action-card-meta">
                  <span class="action-card-account">${account}</span>
                  ${due ? ` * ${due}` : ''}
                </div>
              </div>
            </div>
          `;
        });
      }
    });

    card.innerHTML = `
      <div class="action-card-header">
        <h3 class="action-card-title">Actions Due</h3>
        <span class="action-card-count">${totalCount} items</span>
      </div>
      <div class="action-card-body">
        ${itemsHtml || '<p class="text-muted">No actions due</p>'}
      </div>
      <div class="section-card-footer">
        <a href="/today/actions" class="section-card-link">View all actions -></a>
      </div>
    `;

    return card;
  },

  /**
   * Build waiting-on sidebar card
   * @param {Element} heading - Waiting section heading
   * @returns {HTMLElement} Waiting card element
   */
  buildWaitingCard(heading) {
    const card = document.createElement('div');
    card.className = 'waiting-card animate-in';
    card.style.animationDelay = '0.45s';

    const table = SectionUtils.findNextTable(heading);
    let itemsHtml = '';

    if (table) {
      const rows = SectionUtils.getTableRows(table);
      rows.forEach(row => {
        const cells = row.querySelectorAll('td');
        const who = cells[0]?.textContent.trim() || '';
        const what = cells[1]?.textContent.trim() || '';
        const days = cells[2]?.textContent.trim() || '';

        itemsHtml += `
          <div class="waiting-card-item">
            <span class="waiting-card-who">${who}</span>
            <span class="waiting-card-what">${what}</span>
            <span class="waiting-card-days">${days}</span>
          </div>
        `;
      });
    }

    card.innerHTML = `
      <div class="waiting-card-header">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10"></circle>
          <polyline points="12 6 12 12 16 14"></polyline>
        </svg>
        <h3 class="waiting-card-title">Waiting On</h3>
      </div>
      ${itemsHtml || '<p class="text-muted text-sm">Nothing pending</p>'}
    `;

    return card;
  },

  /**
   * Build focus suggestions sidebar card
   * @param {Element} heading - Focus section heading
   * @returns {HTMLElement} Focus card element
   */
  buildFocusCard(heading) {
    const card = document.createElement('div');
    card.className = 'section-card animate-in';
    card.style.animationDelay = '0.5s';

    const content = SectionUtils.findContent(heading);
    let listHtml = '';

    content.forEach(el => {
      if (el.tagName === 'OL' || el.tagName === 'UL') {
        const items = el.querySelectorAll('li');
        items.forEach((li, i) => {
          listHtml += `
            <div class="action-card-item">
              <div class="action-priority p${i + 1}">${i + 1}</div>
              <div class="action-card-text">
                <div class="action-card-name">${li.innerHTML}</div>
              </div>
            </div>
          `;
        });
      }
    });

    card.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">Suggested Focus</h3>
      </div>
      <div class="section-card-body">
        ${listHtml || '<p class="text-muted">No suggestions</p>'}
      </div>
      <a href="/today/focus" class="section-card-link">View full focus list -></a>
    `;

    return card;
  },

  /**
   * Add click handlers for prep doc links
   * @param {Element} container - Container with prep links
   */
  addPrepDocClickHandlers(container) {
    const prepLinks = container.querySelectorAll('[data-prep], [data-prep-search]');
    prepLinks.forEach(link => {
      link.style.cursor = 'pointer';
      link.addEventListener('click', async (e) => {
        e.preventDefault();

        const prepFile = link.getAttribute('data-prep');
        const prepSearch = link.getAttribute('data-prep-search');

        if (prepFile && window.Router) {
          // Direct file reference
          const routePath = prepFile.replace('.md', '');
          Router.navigate(`/today/${routePath}`);
        } else if (prepSearch && window.Router) {
          // Pattern-based search: try to find matching file
          // Pattern format: "1600-project" meaning files like "01-1600-project-*.md"
          try {
            // Try to fetch directory listing or use known file patterns
            const searchPattern = prepSearch; // e.g., "1600-project"
            // Try common numbering patterns (01, 02, etc.)
            for (let num = 1; num <= 10; num++) {
              const prefix = String(num).padStart(2, '0');
              const testPath = `/today/${prefix}-${searchPattern}`;
              // Navigate and let the router handle 404 gracefully
              Router.navigate(testPath);
              return;
            }
          } catch (err) {
            console.warn('Could not find prep file for pattern:', prepSearch);
          }
        }
      });
    });
  }
};

// Register with TransformRegistry
if (window.TransformRegistry) {
  TransformRegistry.register(OverviewTransform);
}

// Make available globally
window.OverviewTransform = OverviewTransform;
