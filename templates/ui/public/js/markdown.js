/**
 * DailyOS Markdown Utilities
 * Client-side markdown enhancement and processing
 */

const MarkdownUtils = {
  /**
   * Enhance rendered HTML with interactive features and visual transformations
   */
  enhance(container) {
    // Check page types FIRST (before other transforms modify DOM)
    if (this.isOverviewPage(container)) {
      this.transformOverviewPage(container);
      this.addPrepDocClickHandlers(container);
      return;
    }

    if (this.isActionsPage(container)) {
      this.transformActionsPage(container);
      return;
    }

    if (this.isEmailPage(container)) {
      this.transformEmailPage(container);
      return;
    }

    if (this.isFocusPage(container)) {
      this.transformFocusPage(container);
      return;
    }

    if (this.isWeekOverviewPage(container)) {
      this.transformWeekOverviewPage(container);
      return;
    }

    // Default enhancements for other pages
    this.enhanceCheckboxes(container);
    this.enhanceTables(container);
    this.enhanceLinks(container);
    this.enhanceCodeBlocks(container);

    // Visual transformations
    this.transformAlertSections(container);
    this.transformBlockquotes(container);
    this.transformScheduleTables(container);

    // Apply entrance animations
    this.addEntranceAnimations(container);
  },

  /**
   * Check if this is an overview page
   */
  isOverviewPage(container) {
    const h1 = container.querySelector('h1');
    return h1 && h1.textContent.includes('Today:');
  },

  /**
   * Check if this is an actions page
   */
  isActionsPage(container) {
    const h1 = container.querySelector('h1');
    return h1 && h1.textContent.includes('Action Items');
  },

  /**
   * Check if this is an email summary page
   */
  isEmailPage(container) {
    const h1 = container.querySelector('h1');
    return h1 && h1.textContent.includes('Email Summary');
  },

  /**
   * Check if this is a focus page
   */
  isFocusPage(container) {
    const h1 = container.querySelector('h1');
    return h1 && (h1.textContent.includes('Suggested Focus') || h1.textContent.includes('Focus Areas'));
  },

  /**
   * Check if this is a week overview page
   */
  isWeekOverviewPage(container) {
    const h1 = container.querySelector('h1');
    return h1 && h1.textContent.includes('Week Overview');
  },

  /**
   * Make checkboxes interactive
   */
  enhanceCheckboxes(container) {
    const checkboxes = container.querySelectorAll('input[type="checkbox"]');
    checkboxes.forEach((checkbox, index) => {
      // Store original state
      const isChecked = checkbox.checked;

      // Make interactive
      checkbox.disabled = false;
      checkbox.dataset.index = index;

      // Add change listener
      checkbox.addEventListener('change', (e) => {
        const li = e.target.closest('li');
        if (li) {
          const textSpan = li.querySelector('.task-text') || li;
          if (e.target.checked) {
            textSpan.style.textDecoration = 'line-through';
            textSpan.style.color = 'var(--text-muted)';
          } else {
            textSpan.style.textDecoration = 'none';
            textSpan.style.color = '';
          }
        }

        // Store state in localStorage
        this.saveCheckboxState(window.location.pathname, index, e.target.checked);
      });

      // Restore saved state
      const savedState = this.getCheckboxState(window.location.pathname, index);
      if (savedState !== null) {
        checkbox.checked = savedState;
        if (savedState) {
          const li = checkbox.closest('li');
          if (li) {
            const textSpan = li.querySelector('.task-text') || li;
            textSpan.style.textDecoration = 'line-through';
            textSpan.style.color = 'var(--text-muted)';
          }
        }
      }
    });
  },

  /**
   * Save checkbox state to localStorage
   */
  saveCheckboxState(path, index, checked) {
    const key = `dailyos-checkbox-${path}`;
    const states = JSON.parse(localStorage.getItem(key) || '{}');
    states[index] = checked;
    localStorage.setItem(key, JSON.stringify(states));
  },

  /**
   * Get checkbox state from localStorage
   */
  getCheckboxState(path, index) {
    const key = `dailyos-checkbox-${path}`;
    const states = JSON.parse(localStorage.getItem(key) || '{}');
    return states[index] !== undefined ? states[index] : null;
  },

  /**
   * Clear checkbox states for a path (when content changes)
   */
  clearCheckboxStates(path) {
    const key = `dailyos-checkbox-${path}`;
    localStorage.removeItem(key);
  },

  /**
   * Enhance tables with wrapper for scroll
   */
  enhanceTables(container) {
    const tables = container.querySelectorAll('table');
    tables.forEach(table => {
      // Skip if already wrapped
      if (table.parentElement.classList.contains('table-wrapper')) return;

      const wrapper = document.createElement('div');
      wrapper.className = 'table-wrapper';
      wrapper.style.overflowX = 'auto';
      wrapper.style.marginBottom = 'var(--space-6)';

      table.parentNode.insertBefore(wrapper, table);
      wrapper.appendChild(table);
    });
  },

  /**
   * Enhance internal links for SPA navigation
   */
  enhanceLinks(container) {
    const links = container.querySelectorAll('a');
    links.forEach(link => {
      const href = link.getAttribute('href');

      // Skip external links
      if (!href || href.startsWith('http') || href.startsWith('mailto:')) {
        link.target = '_blank';
        link.rel = 'noopener noreferrer';
        return;
      }

      // Handle relative markdown links
      if (href.endsWith('.md')) {
        link.addEventListener('click', (e) => {
          e.preventDefault();
          // Convert .md link to file API call
          const path = this.resolveRelativePath(window.location.pathname, href);
          Router.navigate(`/file?path=${encodeURIComponent(path)}`);
        });
      }

      // Handle internal navigation links
      if (href.startsWith('/')) {
        link.addEventListener('click', (e) => {
          e.preventDefault();
          Router.navigate(href);
        });
      }
    });
  },

  /**
   * Enhance code blocks with copy button
   */
  enhanceCodeBlocks(container) {
    const codeBlocks = container.querySelectorAll('pre');
    codeBlocks.forEach(pre => {
      // Skip if already has copy button
      if (pre.querySelector('.copy-btn')) return;

      const wrapper = document.createElement('div');
      wrapper.style.position = 'relative';

      const copyBtn = document.createElement('button');
      copyBtn.className = 'copy-btn btn btn-ghost btn-icon';
      copyBtn.innerHTML = `
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
          <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
        </svg>
      `;
      copyBtn.style.position = 'absolute';
      copyBtn.style.top = '8px';
      copyBtn.style.right = '8px';
      copyBtn.style.opacity = '0';
      copyBtn.style.transition = 'opacity 0.2s';

      copyBtn.addEventListener('click', async () => {
        const code = pre.querySelector('code');
        if (code) {
          await navigator.clipboard.writeText(code.textContent);
          copyBtn.innerHTML = `
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
              <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
          `;
          setTimeout(() => {
            copyBtn.innerHTML = `
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
              </svg>
            `;
          }, 2000);
        }
      });

      pre.parentNode.insertBefore(wrapper, pre);
      wrapper.appendChild(pre);
      wrapper.appendChild(copyBtn);

      wrapper.addEventListener('mouseenter', () => {
        copyBtn.style.opacity = '1';
      });
      wrapper.addEventListener('mouseleave', () => {
        copyBtn.style.opacity = '0';
      });
    });
  },

  /**
   * Resolve relative path from current location
   */
  resolveRelativePath(currentPath, relativePath) {
    // Remove leading ./
    relativePath = relativePath.replace(/^\.\//, '');

    // Handle ../ navigation
    const parts = currentPath.split('/').filter(Boolean);
    const relParts = relativePath.split('/');

    for (const part of relParts) {
      if (part === '..') {
        parts.pop();
      } else if (part !== '.') {
        parts.push(part);
      }
    }

    return parts.join('/');
  },

  /**
   * Parse frontmatter display
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
   * Transform "Attention Needed" or warning sections into alert boxes
   */
  transformAlertSections(container) {
    const headings = container.querySelectorAll('h2, h3');

    headings.forEach(heading => {
      const text = heading.textContent.toLowerCase();
      let alertType = null;
      let icon = null;

      if (text.includes('attention') || text.includes('warning') || text.includes('overdue')) {
        alertType = 'alert-warning';
        icon = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="20" height="20">
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
          <line x1="12" y1="9" x2="12" y2="13"></line>
          <line x1="12" y1="17" x2="12.01" y2="17"></line>
        </svg>`;
      } else if (text.includes('success') || text.includes('completed') || text.includes('done')) {
        alertType = 'alert-success';
        icon = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="20" height="20">
          <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
          <polyline points="22 4 12 14.01 9 11.01"></polyline>
        </svg>`;
      } else if (text.includes('error') || text.includes('failed') || text.includes('critical')) {
        alertType = 'alert-error';
        icon = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="20" height="20">
          <circle cx="12" cy="12" r="10"></circle>
          <line x1="15" y1="9" x2="9" y2="15"></line>
          <line x1="9" y1="9" x2="15" y2="15"></line>
        </svg>`;
      }

      if (alertType) {
        // Collect content until next heading of same or higher level
        const content = [];
        let sibling = heading.nextElementSibling;
        const headingLevel = parseInt(heading.tagName.charAt(1));

        while (sibling) {
          if (sibling.matches('h1, h2, h3, h4, h5, h6')) {
            const siblingLevel = parseInt(sibling.tagName.charAt(1));
            if (siblingLevel <= headingLevel) break;
          }
          content.push(sibling);
          sibling = sibling.nextElementSibling;
        }

        // Only transform if there's content
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
   * Transform blockquotes into callout boxes
   */
  transformBlockquotes(container) {
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
      }

      const callout = document.createElement('div');
      callout.className = boxClass;
      callout.innerHTML = `
        <span class="callout-box-label">${label}</span>
        <div class="callout-box-content">${quote.innerHTML}</div>
      `;

      quote.parentNode.replaceChild(callout, quote);
    });
  },

  /**
   * Transform schedule/calendar tables into output-preview cards
   */
  transformScheduleTables(container) {
    const tables = container.querySelectorAll('table');

    tables.forEach(table => {
      const headers = Array.from(table.querySelectorAll('th')).map(th => th.textContent.toLowerCase().trim());

      // Check if this looks like a schedule table (has time column)
      const timeColIndex = headers.findIndex(h => h.includes('time') || h.includes('when') || h.includes('start'));
      const titleColIndex = headers.findIndex(h => h.includes('meeting') || h.includes('event') || h.includes('title') || h.includes('what'));
      const typeColIndex = headers.findIndex(h => h.includes('type') || h.includes('category') || h.includes('tag'));

      if (timeColIndex !== -1 && titleColIndex !== -1) {
        const rows = table.querySelectorAll('tbody tr');
        if (rows.length === 0) return;

        const preview = document.createElement('div');
        preview.className = 'output-preview';

        let html = '<div class="output-preview-header">Schedule</div>';

        rows.forEach(row => {
          const cells = row.querySelectorAll('td');
          const time = cells[timeColIndex]?.textContent.trim() || '';
          const title = cells[titleColIndex]?.textContent.trim() || '';
          const type = cells[typeColIndex]?.textContent.toLowerCase().trim() || '';

          // Determine tag type
          let tagClass = 'meeting-tag meeting-tag-internal';
          let tagText = 'internal';
          const isCustomer = type.includes('customer') || type.includes('external') || type.includes('client');
          const isProject = type.includes('project');

          if (isCustomer) {
            tagClass = 'meeting-tag meeting-tag-customer';
            tagText = 'customer';
          } else if (isProject) {
            tagClass = 'meeting-tag meeting-tag-project';
            tagText = 'project';
          }

          html += `
            <div class="meeting-row">
              <span class="meeting-time">${time}</span>
              <span class="meeting-name ${isCustomer ? 'customer' : ''}">${title}</span>
              <span class="${tagClass}">${tagText}</span>
            </div>
          `;
        });

        preview.innerHTML = html;

        // Replace table with preview
        const wrapper = table.closest('.table-wrapper') || table;
        wrapper.parentNode.replaceChild(preview, wrapper);
      }
    });
  },

  /**
   * Enhanced code blocks with terminal styling
   */
  enhanceCodeBlocks(container) {
    const codeBlocks = container.querySelectorAll('pre');

    codeBlocks.forEach(pre => {
      // Skip if already enhanced
      if (pre.closest('.terminal')) return;
      if (pre.querySelector('.copy-btn')) return;

      const code = pre.querySelector('code');
      const language = code?.className.match(/language-(\w+)/)?.[1] || '';
      const content = code?.textContent || pre.textContent;

      // Determine if this looks like a terminal command
      const isTerminal = language === 'bash' || language === 'shell' || language === 'sh' ||
                        content.includes('$') || content.includes('❯') ||
                        content.startsWith('/') || content.startsWith('claude');

      if (isTerminal || language) {
        const terminal = document.createElement('div');
        terminal.className = 'terminal';

        // Apply syntax highlighting for terminal
        let highlightedContent = this.highlightTerminal(content);

        terminal.innerHTML = `
          <div class="terminal-header">
            <div class="terminal-dot terminal-dot-red"></div>
            <div class="terminal-dot terminal-dot-yellow"></div>
            <div class="terminal-dot terminal-dot-green"></div>
            <div class="terminal-title">${language || 'terminal'}</div>
          </div>
          <div class="terminal-body">${highlightedContent}</div>
        `;

        // Add copy button
        const copyBtn = document.createElement('button');
        copyBtn.className = 'copy-btn btn btn-ghost btn-icon';
        copyBtn.style.cssText = 'position: absolute; top: 44px; right: 8px; opacity: 0; transition: opacity 0.2s;';
        copyBtn.innerHTML = `
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        `;

        copyBtn.addEventListener('click', async () => {
          await navigator.clipboard.writeText(content);
          copyBtn.innerHTML = `
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
              <polyline points="20 6 9 17 4 12"></polyline>
            </svg>
          `;
          setTimeout(() => {
            copyBtn.innerHTML = `
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
              </svg>
            `;
          }, 2000);
        });

        const wrapper = document.createElement('div');
        wrapper.style.position = 'relative';
        wrapper.appendChild(terminal);
        wrapper.appendChild(copyBtn);

        wrapper.addEventListener('mouseenter', () => copyBtn.style.opacity = '1');
        wrapper.addEventListener('mouseleave', () => copyBtn.style.opacity = '0');

        pre.parentNode.replaceChild(wrapper, pre);
      } else {
        // Regular code block - just add copy button
        const wrapper = document.createElement('div');
        wrapper.style.position = 'relative';

        const copyBtn = document.createElement('button');
        copyBtn.className = 'copy-btn btn btn-ghost btn-icon';
        copyBtn.innerHTML = `
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        `;
        copyBtn.style.cssText = 'position: absolute; top: 8px; right: 8px; opacity: 0; transition: opacity 0.2s;';

        copyBtn.addEventListener('click', async () => {
          const code = pre.querySelector('code');
          if (code) {
            await navigator.clipboard.writeText(code.textContent);
            copyBtn.innerHTML = `
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                <polyline points="20 6 9 17 4 12"></polyline>
              </svg>
            `;
            setTimeout(() => {
              copyBtn.innerHTML = `
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" width="16" height="16">
                  <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
                  <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
                </svg>
              `;
            }, 2000);
          }
        });

        pre.parentNode.insertBefore(wrapper, pre);
        wrapper.appendChild(pre);
        wrapper.appendChild(copyBtn);

        wrapper.addEventListener('mouseenter', () => copyBtn.style.opacity = '1');
        wrapper.addEventListener('mouseleave', () => copyBtn.style.opacity = '0');
      }
    });
  },

  /**
   * Apply terminal syntax highlighting
   */
  highlightTerminal(content) {
    // Escape HTML
    let html = content
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');

    // Highlight prompts
    html = html.replace(/^(\$|❯|>)\s*/gm, '<span class="prompt">$1 </span>');

    // Highlight comments
    html = html.replace(/(#.*)$/gm, '<span class="comment">$1</span>');

    // Highlight strings
    html = html.replace(/(".*?"|'.*?')/g, '<span class="string">$1</span>');

    // Highlight common commands
    const commands = ['claude', 'cd', 'ls', 'npm', 'git', 'echo', 'cat', 'mkdir', 'rm', 'cp', 'mv'];
    commands.forEach(cmd => {
      const regex = new RegExp(`(^|\\s)(${cmd})(\\s|$)`, 'gm');
      html = html.replace(regex, '$1<span class="command">$2</span>$3');
    });

    // Highlight success messages
    html = html.replace(/(✓|success|done|complete)/gi, '<span class="success">$1</span>');

    // Highlight errors
    html = html.replace(/(✗|error|failed|fail)/gi, '<span class="error">$1</span>');

    return html;
  },

  /**
   * Transform Email page into card-based layout
   */
  transformEmailPage(container) {
    const h1 = container.querySelector('h1');
    const dateMatch = h1.textContent.match(/- (.+)/);
    const dateStr = dateMatch ? dateMatch[1] : 'Email Summary';

    // Find sections
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

    // Count emails - check H2 heading text or count H3 subheadings
    let highCount = 0, mediumCount = 0, archivedCount = 0;

    if (sections.high) {
      // First try to get count from H2 heading: "HIGH Priority Emails (3)"
      const headingMatch = sections.high.textContent.match(/\((\d+)\)/);
      if (headingMatch) {
        highCount = parseInt(headingMatch[1]);
      } else {
        // Fallback: count H3 headings which represent individual emails
        const content = this.findSectionContent(sections.high);
        content.forEach(el => {
          if (el.tagName === 'H3') highCount++;
        });
        // Also check for table (backwards compatibility)
        const table = this.findNextTable(sections.high);
        if (table && highCount === 0) {
          const rows = table.querySelectorAll('tbody tr');
          highCount = rows.length > 0 ? rows.length : Array.from(table.querySelectorAll('tr')).slice(1).length;
        }
      }
    }

    // Build dashboard
    const dashboard = document.createElement('div');
    dashboard.className = 'dashboard';

    dashboard.innerHTML = `
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
   * Build email section card
   */
  buildEmailSection(heading, title, type, delay) {
    const section = document.createElement('div');
    section.className = `section-card section-card-${type === 'high' ? 'warning' : 'week'} animate-in`;
    section.style.animationDelay = `${delay}s`;

    // Include HR elements (email items are separated by ---)
    const content = this.findSectionContent(heading, true);
    let itemsHtml = '';
    let emailIndex = 0;

    // Parse email items - new format uses H3 headings with metadata in following paragraphs
    // Format: ### N. Subject Line
    //         **From**: sender
    //         **Date**: date
    //         > snippet
    //         ---
    for (let i = 0; i < content.length; i++) {
      const el = content[i];

      // Skip HR separators between emails (don't stop main loop)
      if (el.tagName === 'HR') {
        continue;
      }

      // New format: H3 heading with "N. Subject" pattern
      if (el.tagName === 'H3') {
        const h3Text = el.textContent.trim();
        const subjectMatch = h3Text.match(/^\d+\.\s*(.+)$/);

        if (subjectMatch) {
          // This is an email entry
          const subject = subjectMatch[1];
          let from = '';
          let snippet = '';
          let classification = '';

          // Look ahead for **From**, **Date**, blockquote, etc. - stop at next H3, H2, or HR
          for (let j = i + 1; j < content.length; j++) {
            const nextEl = content[j];
            if (!nextEl || !nextEl.tagName) continue;

            // Stop lookahead at section boundaries
            if (nextEl.tagName === 'H3' || nextEl.tagName === 'H2' || nextEl.tagName === 'HR') {
              break;
            }

            if (nextEl.tagName === 'P') {
              const text = nextEl.textContent;
              const html = nextEl.innerHTML;

              // Extract From - could be in same P as Date or separate
              if (html.includes('<strong>From</strong>') || text.match(/^\*\*From\*\*/)) {
                // Try to extract just the From part
                const fromMatch = text.match(/From[:\s]*([^]*?)(?=Date|$)/i);
                if (fromMatch) {
                  from = fromMatch[1].trim();
                  // Clean up any trailing content
                  from = from.replace(/\*\*/g, '').trim();
                }
              }

              // Extract Classification
              if (text.includes('Classification')) {
                classification = text.replace(/Classification[:\s]*/i, '').trim();
              }

              // Handle Snippet label (sometimes it's a paragraph with just "**Snippet**:")
              if (text.includes('Snippet') && !text.includes('>')) {
                continue; // Skip the label, snippet is in blockquote
              }
            }

            // Extract snippet from blockquote
            if (nextEl.tagName === 'BLOCKQUOTE') {
              snippet = nextEl.textContent.trim();
              if (snippet.length > 150) snippet = snippet.substring(0, 150) + '...';
            }
          }

          // Determine type badge based on classification
          let typeClass = 'info';
          if (classification.includes('ACTION')) typeClass = 'action';
          else if (classification.includes('OPPORTUNITY')) typeClass = 'opportunity';
          else if (classification.includes('RISK')) typeClass = 'risk';

          itemsHtml += `
            <div class="email-list-item animate-in-fast" style="animation-delay: ${0.1 + emailIndex * 0.05}s">
              <div class="email-list-badge ${typeClass}"></div>
              <div class="email-list-content">
                <div class="email-list-from">${from}</div>
                <div class="email-list-subject">${subject}</div>
                ${snippet ? `<div class="email-list-snippet">${snippet}</div>` : ''}
              </div>
            </div>
          `;
          emailIndex++;
        } else {
          // Old format: just a group header
          itemsHtml += `<div class="email-group-header">${h3Text}</div>`;
        }
      }

      // Old format fallback: TABLE with From|Subject|Type|Action columns
      if (el.tagName === 'TABLE') {
        const rows = el.querySelectorAll('tbody tr');
        const allRows = rows.length > 0 ? rows : Array.from(el.querySelectorAll('tr')).slice(1);

        allRows.forEach((row, idx) => {
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
                <div class="email-list-from">${from}</div>
                <div class="email-list-subject">${subject}</div>
                ${action ? `<div class="email-list-action">${action}</div>` : ''}
              </div>
              <div class="email-list-type">${emailType}</div>
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
  },

  /**
   * Transform Focus page into card-based layout
   */
  transformFocusPage(container) {
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
   * Build focus section card
   */
  buildFocusSection(heading, priorityNum, delay) {
    const section = document.createElement('div');
    section.className = 'focus-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const title = heading.textContent;
    const content = this.findSectionContent(heading);
    let itemsHtml = '';

    content.forEach(el => {
      // Handle checkboxes in lists
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

      // Handle plain paragraphs
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
   */
  buildEnergySection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const content = this.findSectionContent(heading);
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
  },

  /**
   * Transform Week Overview page into dashboard layout
   */
  transformWeekOverviewPage(container) {
    const h1 = container.querySelector('h1');
    const titleMatch = h1.textContent.match(/Week Overview:\s*(.+)/);
    const weekTitle = titleMatch ? titleMatch[1] : 'Week Overview';

    // Get focus description from first paragraph
    const firstP = container.querySelector('p');
    const focusText = firstP ? firstP.textContent.replace('Your Focus This Week:', '').trim() : '';

    // Find sections
    const sections = {};
    const h2s = container.querySelectorAll('h2');
    h2s.forEach(h2 => {
      const text = h2.textContent.toLowerCase();
      // Match both "Customer Meetings" and "This Week's Meetings"
      if (text.includes('meetings')) sections.meetings = h2;
      if (text.includes('action items')) sections.actions = h2;
      if (text.includes('hygiene')) sections.hygiene = h2;
      if (text.includes('impact')) sections.impact = h2;
      // Match both "Time Block" and "Calendar Blocks"
      if (text.includes('time block') || text.includes('calendar block')) sections.timeBlocks = h2;
      if (text.includes('focus area')) sections.focusAreas = h2;
      if (text.includes('previous week')) sections.previousWeek = h2;
    });

    // Count stats
    const stats = this.countWeekStats(container, sections);

    // Build dashboard
    const dashboard = document.createElement('div');
    dashboard.className = 'dashboard';

    dashboard.innerHTML = `
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

    // Two column layout
    const grid = document.createElement('div');
    grid.className = 'dashboard-grid';

    const mainCol = document.createElement('div');
    mainCol.className = 'dashboard-main';

    const sideCol = document.createElement('div');
    sideCol.className = 'dashboard-sidebar';

    // Build customer meetings section
    if (sections.meetings) {
      mainCol.appendChild(this.buildWeekMeetingsSection(sections.meetings, 0.3));
    }

    // Build actions section
    if (sections.actions) {
      mainCol.appendChild(this.buildWeekActionsSection(sections.actions, 0.35));
    }

    // Build hygiene alerts section
    if (sections.hygiene) {
      sideCol.appendChild(this.buildHygieneSection(sections.hygiene, 0.3));
    }

    // Build time blocks section
    if (sections.timeBlocks) {
      sideCol.appendChild(this.buildTimeBlocksSection(sections.timeBlocks, 0.35));
    }

    // Build focus areas section
    if (sections.focusAreas) {
      sideCol.appendChild(this.buildWeekFocusSection(sections.focusAreas, 0.4));
    }

    grid.appendChild(mainCol);
    grid.appendChild(sideCol);
    dashboard.appendChild(grid);

    container.innerHTML = '';
    container.appendChild(dashboard);

    // Add click handlers for section links
    this.enhanceLinks(container);
  },

  /**
   * Count week overview stats
   */
  countWeekStats(container, sections) {
    const stats = { meetings: 0, overdue: 0, dueThisWeek: 0, hygieneAlerts: 0 };

    // Count meetings from table
    if (sections.meetings) {
      const table = this.findNextTable(sections.meetings);
      if (table) {
        const rows = table.querySelectorAll('tbody tr');
        stats.meetings = rows.length > 0 ? rows.length : Array.from(table.querySelectorAll('tr')).slice(1).length;
      }
    }

    // Count actions - look for H3 headings with (N) pattern
    if (sections.actions) {
      const content = this.findSectionContent(sections.actions);
      content.forEach(el => {
        if (el.tagName === 'H3') {
          const text = el.textContent;
          const textLower = text.toLowerCase();
          // Try to extract count from "(N)" pattern in heading
          const countMatch = text.match(/\((\d+)\)/);
          if (countMatch) {
            const count = parseInt(countMatch[1]);
            if (textLower.includes('overdue')) stats.overdue += count;
            else if (textLower.includes('due this week')) stats.dueThisWeek += count;
          } else {
            // Fallback: count actual list items
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

    // Count hygiene alerts
    if (sections.hygiene) {
      const content = this.findSectionContent(sections.hygiene);
      content.forEach(el => {
        if (el.tagName === 'TABLE') {
          const rows = el.querySelectorAll('tbody tr');
          stats.hygieneAlerts += rows.length > 0 ? rows.length : Array.from(el.querySelectorAll('tr')).slice(1).length;
        }
      });
    }

    return stats;
  },

  /**
   * Build week meetings section
   */
  buildWeekMeetingsSection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const table = this.findNextTable(heading);
    let itemsHtml = '';

    if (table) {
      const rows = table.querySelectorAll('tbody tr');
      const allRows = rows.length > 0 ? rows : Array.from(table.querySelectorAll('tr')).slice(1);

      let currentDay = '';
      allRows.forEach((row, i) => {
        const cells = row.querySelectorAll('td');
        const day = cells[0]?.textContent.trim() || '';
        const time = cells[1]?.textContent.trim() || '';
        const account = cells[2]?.textContent.trim() || '';
        const ring = cells[3]?.textContent.trim() || '';
        const status = cells[4]?.textContent.trim() || '';

        // Determine tier class (generic tier-1 through tier-4 system)
        let tierClass = 'tier-4';
        const tierLower = ring.toLowerCase();
        if (tierLower.includes('tier-1') || tierLower.includes('tier 1')) tierClass = 'tier-1';
        else if (tierLower.includes('tier-2') || tierLower.includes('tier 2')) tierClass = 'tier-2';
        else if (tierLower.includes('tier-3') || tierLower.includes('tier 3')) tierClass = 'tier-3';
        else if (tierLower.includes('project')) tierClass = 'project';

        // Status indicator
        const needsPrep = status.includes('Needs prep') || status.includes('⚠️');
        const statusClass = needsPrep ? 'needs-prep' : 'ready';

        // Day header if changed
        if (day !== currentDay) {
          currentDay = day;
          itemsHtml += `<div class="week-day-header">${day}</div>`;
        }

        itemsHtml += `
          <div class="week-meeting-item animate-in-fast" style="animation-delay: ${0.1 + i * 0.03}s">
            <div class="week-meeting-time">${time}</div>
            <div class="week-meeting-content">
              <div class="week-meeting-account">${account}</div>
              <div class="week-meeting-meta">
                <span class="ring-badge ${tierClass}">${ring}</span>
                <span class="prep-status ${statusClass}">${needsPrep ? '<span class="prep-icon warning"></span> Needs prep' : '<span class="prep-icon ready"></span> Ready'}</span>
              </div>
            </div>
          </div>
        `;
      });
    }

    // Get note if exists
    const content = this.findSectionContent(heading);
    let noteHtml = '';
    content.forEach(el => {
      if (el.tagName === 'P' && el.textContent.includes('Note:')) {
        noteHtml = `<div class="section-note">${el.innerHTML}</div>`;
      }
    });

    section.innerHTML = `
      <div class="section-card-header">
        <h3 class="section-card-title">Customer Meetings</h3>
        <a href="/today/week-meetings" class="section-card-link" data-route="/today/week-meetings">View All →</a>
      </div>
      <div class="section-card-body">
        ${itemsHtml || '<p class="text-muted text-center">No meetings this week</p>'}
        ${noteHtml}
      </div>
    `;

    return section;
  },

  /**
   * Build week actions section
   */
  buildWeekActionsSection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const content = this.findSectionContent(heading);
    let itemsHtml = '';

    content.forEach(el => {
      if (el.tagName === 'H3') {
        const groupTitle = el.textContent;
        const isOverdue = groupTitle.toLowerCase().includes('overdue');
        itemsHtml += `<div class="action-group-header ${isOverdue ? 'overdue' : ''}">${groupTitle}</div>`;
      }

      if (el.tagName === 'UL') {
        const items = el.querySelectorAll(':scope > li');
        items.forEach((li, i) => {
          const text = li.innerHTML;
          const hasCheckbox = li.querySelector('input[type="checkbox"]');

          itemsHtml += `
            <div class="week-action-item">
              ${hasCheckbox ? '<input type="checkbox" class="week-action-checkbox" />' : '<span class="week-action-bullet">•</span>'}
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
        <a href="/today/week-actions" class="section-card-link" data-route="/today/week-actions">View All →</a>
      </div>
      <div class="section-card-body">
        ${itemsHtml || '<p class="text-muted text-center">No actions</p>'}
      </div>
    `;

    return section;
  },

  /**
   * Build hygiene alerts section
   */
  buildHygieneSection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const content = this.findSectionContent(heading);
    let itemsHtml = '';
    let currentLevel = 'info';

    content.forEach(el => {
      if (el.tagName === 'H3') {
        const text = el.textContent;
        if (text.includes('🔴') || text.toLowerCase().includes('critical')) currentLevel = 'critical';
        else if (text.includes('🟡') || text.toLowerCase().includes('attention')) currentLevel = 'warning';
        else if (text.includes('🟢') || text.toLowerCase().includes('healthy')) currentLevel = 'healthy';

        itemsHtml += `<div class="hygiene-level-header ${currentLevel}">${text}</div>`;
      }

      // Handle "✅ No critical alerts" type paragraphs
      if (el.tagName === 'P' && (el.textContent.includes('None') || el.textContent.includes('✅') || el.textContent.includes('No '))) {
        itemsHtml += `<p class="text-muted text-sm">${el.textContent}</p>`;
      }

      // Handle TABLE format
      if (el.tagName === 'TABLE') {
        const rows = el.querySelectorAll('tbody tr');
        const allRows = rows.length > 0 ? rows : Array.from(el.querySelectorAll('tr')).slice(1);

        allRows.forEach(row => {
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

      // Handle UL format (new markdown structure)
      if (el.tagName === 'UL') {
        const items = el.querySelectorAll('li');
        items.forEach(li => {
          const text = li.textContent;
          // Parse "**Account Name** - Issue description" format
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
            // Simple list item
            const icon = currentLevel === 'healthy' ? '✓' : (currentLevel === 'warning' ? '⚠' : '•');
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
        <a href="/today/week-hygiene" class="section-card-link" data-route="/today/week-hygiene">View All →</a>
      </div>
      <div class="section-card-body">
        ${itemsHtml || '<p class="text-muted text-center">All accounts healthy</p>'}
      </div>
    `;

    return section;
  },

  /**
   * Build time blocks section
   */
  buildTimeBlocksSection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const content = this.findSectionContent(heading);
    let itemsHtml = '';

    content.forEach(el => {
      // Handle table format: | Block | Day | Duration |
      if (el.tagName === 'TABLE') {
        const rows = el.querySelectorAll('tbody tr');
        const allRows = rows.length > 0 ? rows : Array.from(el.querySelectorAll('tr')).slice(1);

        allRows.forEach(row => {
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

      // Handle old format: paragraphs with <strong>
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
   * Build week focus areas section
   */
  buildWeekFocusSection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const content = this.findSectionContent(heading);
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
        <a href="/today/week-focus" class="section-card-link" data-route="/today/week-focus">View All →</a>
      </div>
      <div class="section-card-body">
        ${itemsHtml || '<p class="text-muted text-center">No focus areas defined</p>'}
      </div>
    `;

    return section;
  },

  /**
   * Transform Actions page into card-based layout
   */
  transformActionsPage(container) {
    const h1 = container.querySelector('h1');
    const dateMatch = h1.textContent.match(/- (.+)/);
    const dateStr = dateMatch ? dateMatch[1] : 'Actions';

    // Find all h2 sections
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

    // Count items for stats
    const stats = this.countActionItems(container, sections);

    // Build dashboard
    const dashboard = document.createElement('div');
    dashboard.className = 'dashboard';

    dashboard.innerHTML = `
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

    // Create main content area
    const mainContent = document.createElement('div');
    mainContent.className = 'actions-layout';

    // Build overdue section if has items
    if (sections.overdue && stats.overdue > 0) {
      mainContent.appendChild(this.buildActionSection(sections.overdue, 'Overdue', 'warning', 0.3));
    }

    // Build due today section
    if (sections.dueToday) {
      mainContent.appendChild(this.buildActionSection(sections.dueToday, 'Due Today', 'today', 0.35));
    }

    // Build due this week section
    if (sections.dueThisWeek) {
      mainContent.appendChild(this.buildActionSection(sections.dueThisWeek, 'Due This Week', 'week', 0.4));
    }

    // Build due later section
    if (sections.dueLater) {
      mainContent.appendChild(this.buildActionSection(sections.dueLater, 'Due Later', 'later', 0.45));
    }

    // Build waiting section
    if (sections.waiting) {
      mainContent.appendChild(this.buildWaitingSection(sections.waiting, 0.5));
    }

    dashboard.appendChild(mainContent);
    container.innerHTML = '';
    container.appendChild(dashboard);
  },

  /**
   * Count action items from page content
   */
  countActionItems(container, sections) {
    const stats = { overdue: 0, dueToday: 0, dueThisWeek: 0, waiting: 0 };

    // Count checkbox items in each section
    if (sections.overdue) {
      const content = this.findSectionContent(sections.overdue);
      content.forEach(el => {
        if (el.tagName === 'UL') {
          // Only count top-level items, not nested metadata bullets
          stats.overdue += el.querySelectorAll(':scope > li').length;
        }
      });
      // Check if "No overdue items" text
      if (stats.overdue === 0) {
        const text = content.map(el => el.textContent).join(' ').toLowerCase();
        if (text.includes('no overdue')) stats.overdue = 0;
      }
    }

    if (sections.dueToday) {
      const content = this.findSectionContent(sections.dueToday);
      content.forEach(el => {
        if (el.tagName === 'UL') {
          // Only count top-level items, not nested metadata bullets
          stats.dueToday += el.querySelectorAll(':scope > li').length;
        }
      });
    }

    if (sections.dueThisWeek) {
      const content = this.findSectionContent(sections.dueThisWeek);
      content.forEach(el => {
        if (el.tagName === 'UL') {
          // Only count top-level items with checkboxes (action items, not sub-items)
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
      const table = this.findNextTable(sections.waiting);
      if (table) {
        const rows = table.querySelectorAll('tbody tr');
        stats.waiting = rows.length > 0 ? rows.length : Array.from(table.querySelectorAll('tr')).slice(1).length;
      }
    }

    return stats;
  },

  /**
   * Build an action section card
   */
  buildActionSection(heading, title, type, delay) {
    const section = document.createElement('div');
    section.className = `section-card section-card-${type} animate-in`;
    section.style.animationDelay = `${delay}s`;

    const content = this.findSectionContent(heading);
    let itemsHtml = '';
    let hasItems = false;

    content.forEach(el => {
      // Handle direct UL with action items
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
      // Handle H3 + UL pattern (grouped items)
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
      // Handle plain text like "No items"
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
   * Expected markdown format:
   * - [ ] **Title** - Account Name - Due: 2026-01-24 (X days overdue)
   *   - **Context**: Why this task exists...
   *   - **Source**: Where this came from...
   */
  parseActionItem(li, index) {
    const text = li.innerHTML;
    const plainText = li.textContent;

    // Skip if this is just a sub-item (detail line)
    if (!text.includes('checkbox') && !text.includes('[ ]') && !li.querySelector('strong')) {
      // This might be a nested item
      if (li.parentElement.parentElement.tagName === 'LI') {
        return null;
      }
    }

    // Extract title from bold text
    const titleMatch = text.match(/<strong>([^<]+)<\/strong>/);
    const title = titleMatch ? titleMatch[1] : li.firstChild?.textContent?.trim() || 'Untitled';

    // Extract account/project from line text (between title and "Due:")
    // Format: **Title** - Account Name - Due: ...
    let account = '';
    let due = '';
    let overdue = '';
    let context = '';
    let source = '';
    let owner = '';
    let priority = '';

    // Parse the main line for account and due date
    // The format is: **Title** - Account - Due: YYYY-MM-DD (X days overdue)
    const lineText = plainText.split('\n')[0]; // Get first line only

    // Extract account (text between title and " - Due:")
    const titleEnd = titleMatch ? lineText.indexOf(titleMatch[1]) + titleMatch[1].length : 0;
    const dueStart = lineText.indexOf(' - Due:');
    if (dueStart > titleEnd) {
      // Account is between title and Due
      let accountText = lineText.substring(titleEnd, dueStart).trim();
      // Remove leading " - " if present
      if (accountText.startsWith(' - ')) accountText = accountText.substring(3);
      if (accountText.startsWith('- ')) accountText = accountText.substring(2);
      account = accountText.trim();
    }

    // Extract due date and overdue status
    const dueMatch = lineText.match(/Due:\s*(\d{4}-\d{2}-\d{2})/);
    if (dueMatch) {
      due = dueMatch[1];
    }
    const overdueMatch = lineText.match(/\((\d+)\s*days?\s*overdue\)/i);
    if (overdueMatch) {
      overdue = overdueMatch[1] + ' days overdue';
      priority = 'Overdue';
    }

    // Look for sub-items (ul inside li) for context/source
    const subList = li.querySelector('ul');
    if (subList) {
      const subItems = subList.querySelectorAll('li');
      subItems.forEach(sub => {
        const subText = sub.textContent.trim();
        // Check for Context: (with or without bold)
        if (subText.toLowerCase().startsWith('context:')) {
          context = subText.replace(/^context:\s*/i, '').trim();
        } else if (sub.innerHTML.includes('<strong>Context</strong>')) {
          context = subText.replace(/^Context:\s*/i, '').trim();
        }
        // Check for Source:
        if (subText.toLowerCase().startsWith('source:')) {
          source = subText.replace(/^source:\s*/i, '').trim();
        } else if (sub.innerHTML.includes('<strong>Source</strong>')) {
          source = subText.replace(/^Source:\s*/i, '').trim();
        }
        // Check for Owner:
        if (subText.toLowerCase().includes('owner:')) {
          const ownerMatch = subText.match(/owner:\s*([^,\n]+)/i);
          if (ownerMatch) owner = ownerMatch[1].trim();
        }
        // Legacy format support
        if (subText.includes('Account:')) account = subText.split('Account:')[1]?.split('-')[0]?.trim() || account;
        if (subText.includes('Priority:')) priority = subText.split('Priority:')[1]?.trim() || priority;
      });
    }

    // Determine priority class and badge text
    let priorityClass = 'p2';
    let priorityText = 'P2';
    if (overdue) {
      priorityClass = 'overdue';
      priorityText = overdue;
    } else if (priority) {
      priorityClass = priority.toLowerCase().replace(/\s/g, '');
      priorityText = priority;
    }

    // Build description with context and source
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
   * Build waiting on section
   */
  buildWaitingSection(heading, delay) {
    const section = document.createElement('div');
    section.className = 'section-card animate-in';
    section.style.animationDelay = `${delay}s`;

    const table = this.findNextTable(heading);
    let itemsHtml = '';

    if (table) {
      const rows = table.querySelectorAll('tbody tr');
      const allRows = rows.length > 0 ? rows : Array.from(table.querySelectorAll('tr')).slice(1);

      allRows.forEach((row, i) => {
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
  },

  /**
   * Transform Overview page into dashboard layout
   */
  transformOverviewPage(container) {
    // Check if this is an overview page by looking for the title pattern
    const h1 = container.querySelector('h1');
    if (!h1 || !h1.textContent.includes('Today:')) return;

    // Parse the date from title
    const dateMatch = h1.textContent.match(/Today:\s*(.+)/);
    const dateStr = dateMatch ? dateMatch[1] : 'Today';

    // Find key sections
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

    // Build dashboard structure
    const dashboard = document.createElement('div');
    dashboard.className = 'dashboard';

    // Dashboard header with stats
    const stats = this.extractOverviewStats(container, sections);
    dashboard.innerHTML = `
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

    // Create two-column grid
    const grid = document.createElement('div');
    grid.className = 'dashboard-grid';

    const mainCol = document.createElement('div');
    mainCol.className = 'dashboard-main';

    const sideCol = document.createElement('div');
    sideCol.className = 'dashboard-sidebar';

    // Transform schedule into timeline
    if (sections.schedule) {
      const scheduleCard = this.buildScheduleTimeline(sections.schedule);
      mainCol.appendChild(scheduleCard);
    }

    // Transform customer meetings into cards
    if (sections.customerMeetings) {
      const meetingsCard = this.buildCustomerMeetingsSection(sections.customerMeetings);
      mainCol.appendChild(meetingsCard);
    }

    // Build sidebar cards
    if (sections.email) {
      const emailCard = this.buildEmailSummaryCard(sections.email);
      sideCol.appendChild(emailCard);
    }

    if (sections.actions) {
      const actionsCard = this.buildActionsCard(sections.actions);
      sideCol.appendChild(actionsCard);
    }

    if (sections.waiting) {
      const waitingCard = this.buildWaitingCard(sections.waiting);
      sideCol.appendChild(waitingCard);
    }

    if (sections.focus) {
      const focusCard = this.buildFocusCard(sections.focus);
      sideCol.appendChild(focusCard);
    }

    grid.appendChild(mainCol);
    grid.appendChild(sideCol);
    dashboard.appendChild(grid);

    // Replace entire content
    container.innerHTML = '';
    container.appendChild(dashboard);
  },

  /**
   * Extract stats from overview content
   */
  extractOverviewStats(container, sections) {
    const stats = {
      totalMeetings: 0,
      customerMeetings: 0,
      actionsDue: 0,
      highPriorityEmails: 0,
      waitingOn: 0
    };

    // Count schedule rows
    if (sections.schedule) {
      const table = this.findNextTable(sections.schedule);
      if (table) {
        // Handle tables with or without tbody
        const rows = table.querySelectorAll('tbody tr') || table.querySelectorAll('tr:not(:first-child)');
        // If no tbody rows, fall back to all tr except header
        const allRows = rows.length > 0 ? rows : Array.from(table.querySelectorAll('tr')).slice(1);
        stats.totalMeetings = allRows.length;
        allRows.forEach(row => {
          const typeCell = row.querySelector('td:nth-child(3)');
          if (typeCell && typeCell.textContent.toLowerCase().includes('customer')) {
            stats.customerMeetings++;
          }
        });
      }
    }

    // Count actions - look for H3 headings with (N) pattern or UL items
    if (sections.actions) {
      const content = this.findSectionContent(sections.actions);
      let actionCount = 0;
      content.forEach(el => {
        // Check for H3 headings with count in parentheses: "Overdue (10)", "Due Today (0)"
        if (el.tagName === 'H3') {
          const match = el.textContent.match(/\((\d+)\)/);
          if (match) {
            actionCount += parseInt(match[1]);
          }
        }
        // Fallback: count actual checkbox items in UL
        if (el.tagName === 'UL') {
          const checkboxItems = el.querySelectorAll('li input[type="checkbox"]');
          if (checkboxItems.length > 0) {
            actionCount += checkboxItems.length;
          }
        }
        // Also check tables for backwards compatibility
        if (el.tagName === 'TABLE') {
          const rows = el.querySelectorAll('tbody tr');
          const allRows = rows.length > 0 ? rows : Array.from(el.querySelectorAll('tr')).slice(1);
          actionCount += allRows.length;
        }
      });
      stats.actionsDue = actionCount;
    }

    // Count high priority emails from the section heading or h3 within
    if (sections.email) {
      // Check heading itself
      let match = sections.email.textContent.match(/\((\d+)\)/);
      if (!match) {
        // Look for h3 with count in section content
        const content = this.findSectionContent(sections.email);
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
      const table = this.findNextTable(sections.waiting);
      if (table) {
        const rows = table.querySelectorAll('tbody tr');
        const allRows = rows.length > 0 ? rows : Array.from(table.querySelectorAll('tr')).slice(1);
        stats.waitingOn = allRows.length;
      }
    }

    return stats;
  },

  /**
   * Find next table after a heading
   */
  findNextTable(heading) {
    let sibling = heading.nextElementSibling;
    while (sibling) {
      if (sibling.tagName === 'TABLE') return sibling;
      if (sibling.querySelector('table')) return sibling.querySelector('table');
      if (sibling.matches('h2, h1')) break;
      sibling = sibling.nextElementSibling;
    }
    return null;
  },

  /**
   * Find content between heading and next h2
   * @param {Element} heading - The heading element to start from
   * @param {boolean} includeHr - If true, include HR elements and continue past them (default: false)
   */
  findSectionContent(heading, includeHr = false) {
    const content = [];
    let sibling = heading.nextElementSibling;
    while (sibling && !sibling.matches('h2, h1')) {
      // Stop at HR unless includeHr is true
      if (sibling.tagName === 'HR' && !includeHr) {
        break;
      }
      content.push(sibling);
      sibling = sibling.nextElementSibling;
    }
    return content;
  },

  /**
   * Build schedule timeline
   */
  buildScheduleTimeline(heading) {
    const card = document.createElement('div');
    card.className = 'section-card animate-in';
    card.style.animationDelay = '0.3s';

    const table = this.findNextTable(heading);
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

    // Handle tables with or without tbody
    const tbodyRows = table.querySelectorAll('tbody tr');
    const rows = tbodyRows.length > 0 ? tbodyRows : Array.from(table.querySelectorAll('tr')).slice(1);
    let timelineHtml = '';

    rows.forEach((row, i) => {
      const cells = row.querySelectorAll('td');
      const time = cells[0]?.textContent.trim() || '';
      const event = cells[1]?.textContent.trim() || '';
      const type = cells[2]?.textContent.trim().toLowerCase() || '';
      const prep = cells[3]?.textContent.trim() || '';

      const isCustomer = type.includes('customer');
      const isInternal = type.includes('internal');
      const isPersonal = type.includes('personal');

      const typeClass = isCustomer ? 'customer' : isInternal ? 'internal' : 'personal';
      const eventClean = event.replace(/\*\*/g, '');

      timelineHtml += `
        <div class="timeline-item animate-in-fast" style="animation-delay: ${0.35 + i * 0.05}s">
          <div class="timeline-marker">
            <div class="timeline-dot ${typeClass}"></div>
            <div class="timeline-time">${time}</div>
          </div>
          <div class="timeline-content ${typeClass}">
            <div class="timeline-title ${typeClass}">${eventClean}</div>
            <div class="timeline-meta">
              <span class="meeting-tag meeting-tag-${isCustomer ? 'customer' : isInternal ? 'internal' : 'project'}">${type}</span>
            </div>
            ${prep && prep !== '-' ? `
              <div class="timeline-prep">
                <a href="#" class="timeline-prep-link" data-prep="${prep}">
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
   * Build customer meetings section
   */
  buildCustomerMeetingsSection(heading) {
    const card = document.createElement('div');
    card.className = 'section-card animate-in';
    card.style.animationDelay = '0.4s';

    const content = this.findSectionContent(heading);
    let cardsHtml = '';

    // Find h3s which are individual meeting headers
    content.forEach(el => {
      if (el.tagName === 'H3') {
        const meetingName = el.textContent.replace(/\*\*/g, '');
        const details = {};

        // Get the ul following this h3
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

        // Determine tier class (generic tier-1 through tier-4 system)
        let tierClass = 'tier-4';
        const tierLower = (details.ring || '').toLowerCase();
        if (tierLower.includes('tier-1') || tierLower.includes('tier 1')) tierClass = 'tier-1';
        else if (tierLower.includes('tier-2') || tierLower.includes('tier 2')) tierClass = 'tier-2';
        else if (tierLower.includes('tier-3') || tierLower.includes('tier 3')) tierClass = 'tier-3';

        cardsHtml += `
          <div class="meeting-card">
            <div class="meeting-card-header">
              <h4 class="meeting-card-title">${meetingName}</h4>
              <span class="ring-badge ${tierClass}">${details.ring || 'Unknown'}</span>
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
   * Build email summary card - uses action-card structure for consistency
   */
  buildEmailSummaryCard(heading) {
    const card = document.createElement('div');
    card.className = 'action-card animate-in';
    card.style.animationDelay = '0.35s';

    // Extract count from heading or h3 within section
    let count = 0;
    const countMatch = heading.textContent.match(/\((\d+)\)/);
    if (countMatch) {
      count = parseInt(countMatch[1]);
    } else {
      const content = this.findSectionContent(heading);
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

    // Find the table with email items
    const content = this.findSectionContent(heading);
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

    // Helper to escape HTML characters
    const escapeHtml = (str) => str
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');

    if (table) {
      const tbodyRows = table.querySelectorAll('tbody tr');
      const rows = tbodyRows.length > 0 ? tbodyRows : Array.from(table.querySelectorAll('tr')).slice(1);
      rows.forEach((row, i) => {
        if (i >= 4) return;
        const cells = row.querySelectorAll('td');
        const from = escapeHtml(cells[0]?.textContent.trim() || '');
        const subject = escapeHtml(cells[1]?.textContent.trim() || '');

        itemsHtml += `
          <div class="action-card-item">
            <div class="action-priority email">●</div>
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
        <a href="/today/email" class="section-card-link">View all emails →</a>
      </div>
    `;

    return card;
  },

  /**
   * Build actions card
   */
  buildActionsCard(heading) {
    const card = document.createElement('div');
    card.className = 'action-card animate-in';
    card.style.animationDelay = '0.4s';

    // Find h3s within this section for groupings
    const content = this.findSectionContent(heading);
    let itemsHtml = '';
    let totalCount = 0;

    content.forEach(el => {
      if (el.tagName === 'TABLE') {
        const rows = el.querySelectorAll('tbody tr');
        totalCount += rows.length;
        rows.forEach((row, i) => {
          if (i >= 5) return; // Max 5 items
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
                  ${due ? ` · ${due}` : ''}
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
        <a href="/today/actions" class="section-card-link">View all actions →</a>
      </div>
    `;

    return card;
  },

  /**
   * Build waiting card
   */
  buildWaitingCard(heading) {
    const card = document.createElement('div');
    card.className = 'waiting-card animate-in';
    card.style.animationDelay = '0.45s';

    const table = this.findNextTable(heading);
    let itemsHtml = '';

    if (table) {
      const rows = table.querySelectorAll('tbody tr');
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
   * Build focus suggestions card
   */
  buildFocusCard(heading) {
    const card = document.createElement('div');
    card.className = 'section-card animate-in';
    card.style.animationDelay = '0.5s';

    const content = this.findSectionContent(heading);
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
      <a href="/today/focus" class="section-card-link">View full focus list →</a>
    `;

    return card;
  },

  /**
   * Add click handlers for prep doc links
   */
  addPrepDocClickHandlers(container) {
    // Find all elements with data-prep attribute
    const prepLinks = container.querySelectorAll('[data-prep]');
    prepLinks.forEach(link => {
      link.style.cursor = 'pointer';
      link.addEventListener('click', (e) => {
        e.preventDefault();
        const prepFile = link.getAttribute('data-prep');
        if (prepFile) {
          // Navigate using /today/ route which has flexible filename matching
          const routePath = prepFile.replace('.md', '');
          Router.navigate(`/today/${routePath}`);
        }
      });
    });
  },

  /**
   * Add entrance animations to content elements
   */
  addEntranceAnimations(container) {
    // Animate headings
    const headings = container.querySelectorAll('h1, h2, h3');
    headings.forEach((h, i) => {
      h.classList.add('animate-in');
      h.style.animationDelay = `${i * 0.05}s`;
    });

    // Animate cards and alerts with stagger
    const cards = container.querySelectorAll('.card, .alert, .output-preview, .callout-box, .terminal');
    cards.forEach((card, i) => {
      card.classList.add('animate-in');
      card.style.animationDelay = `${0.1 + i * 0.08}s`;
    });

    // Animate folder items with stagger
    const folderItems = container.querySelectorAll('.folder-item');
    folderItems.forEach((item, i) => {
      item.classList.add('animate-in-fast');
      item.style.animationDelay = `${0.1 + i * 0.04}s`;
    });

    // Animate file list items
    const fileItems = container.querySelectorAll('.file-list-item');
    fileItems.forEach((item, i) => {
      item.classList.add('animate-in-fast');
      item.style.animationDelay = `${0.1 + i * 0.03}s`;
    });

    // Animate meeting rows
    const meetingRows = container.querySelectorAll('.meeting-row');
    meetingRows.forEach((row, i) => {
      row.classList.add('animate-in-fast');
      row.style.animationDelay = `${0.15 + i * 0.05}s`;
    });
  }
};

// Export for use in other modules
window.MarkdownUtils = MarkdownUtils;
