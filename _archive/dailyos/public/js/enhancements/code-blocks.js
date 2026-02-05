/**
 * DailyOS Code Block Enhancement
 * Terminal styling and copy functionality
 *
 * Features:
 * - Terminal-style UI with traffic light dots
 * - Syntax highlighting for common commands
 * - Copy-to-clipboard buttons with feedback
 * - Automatic detection of bash/shell blocks
 *
 * @module enhancements/code-blocks
 * @requires CopyButton
 */

const EnhanceCodeBlocks = {
  /**
   * Enhance code blocks with terminal styling and copy buttons
   * @param {Element} container
   */
  apply(container) {
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
                        content.includes('$') || content.includes('\u276F') ||
                        content.startsWith('/') || content.startsWith('claude');

      if (isTerminal || language) {
        this.createTerminalBlock(pre, content, language);
      } else {
        this.addCopyButton(pre);
      }
    });
  },

  /**
   * Create terminal-styled code block
   * @param {HTMLPreElement} pre
   * @param {string} content
   * @param {string} language
   */
  createTerminalBlock(pre, content, language) {
    const terminal = document.createElement('div');
    terminal.className = 'terminal';

    const highlightedContent = this.highlightTerminal(content);

    terminal.innerHTML = `
      <div class="terminal-header">
        <div class="terminal-dot terminal-dot-red"></div>
        <div class="terminal-dot terminal-dot-yellow"></div>
        <div class="terminal-dot terminal-dot-green"></div>
        <div class="terminal-title">${language || 'terminal'}</div>
      </div>
      <div class="terminal-body">${highlightedContent}</div>
    `;

    const wrapper = document.createElement('div');
    wrapper.style.position = 'relative';
    wrapper.appendChild(terminal);

    // Add copy button using utility
    if (window.CopyButton) {
      CopyButton.attach(wrapper, () => content, { top: '44px', right: '8px' });
    }

    pre.parentNode.replaceChild(wrapper, pre);
  },

  /**
   * Add copy button to regular code block
   * @param {HTMLPreElement} pre
   */
  addCopyButton(pre) {
    const wrapper = document.createElement('div');
    wrapper.style.position = 'relative';

    pre.parentNode.insertBefore(wrapper, pre);
    wrapper.appendChild(pre);

    // Add copy button using utility
    if (window.CopyButton) {
      const code = pre.querySelector('code');
      CopyButton.attach(wrapper, () => code?.textContent || pre.textContent);
    }
  },

  /**
   * Apply terminal syntax highlighting
   * @param {string} content
   * @returns {string}
   */
  highlightTerminal(content) {
    // Escape HTML
    let html = content
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');

    // Highlight prompts
    html = html.replace(/^(\$|\u276F|>)\s*/gm, '<span class="prompt">$1 </span>');

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
    html = html.replace(/(\u2713|success|done|complete)/gi, '<span class="success">$1</span>');

    // Highlight errors
    html = html.replace(/(\u2717|error|failed|fail)/gi, '<span class="error">$1</span>');

    return html;
  }
};

// Make available globally
window.EnhanceCodeBlocks = EnhanceCodeBlocks;
