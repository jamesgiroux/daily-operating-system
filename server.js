/**
 * DailyOS Web Server
 * Config-driven Express server for rendering markdown files from workspace directory
 */

const express = require('express');
const path = require('path');
const fs = require('fs').promises;
const matter = require('gray-matter');
const { marked } = require('marked');

const app = express();
const PORT = process.env.PORT || 5050;

// Configure marked for GFM
marked.setOptions({
  gfm: true,
  breaks: true,
  headerIds: true,
  mangle: false
});

// Load configuration
let config;
try {
  config = require('./config/config.json');
} catch (error) {
  console.error('Failed to load config.json:', error.message);
  console.error('Please ensure config/config.json exists');
  process.exit(1);
}

// Base directory for workspace content (defaults to parent of _ui)
const BASE_DIR = config.workspace.basePath
  ? path.resolve(config.workspace.basePath)
  : path.resolve(__dirname, '..');

// Serve static files
app.use(express.static(path.join(__dirname, 'public')));

// API Routes

/**
 * Get configuration (for frontend)
 */
app.get('/api/config', (req, res) => {
  res.json({
    success: true,
    config: {
      workspace: config.workspace,
      sections: config.sections,
      today: config.today,
      features: config.features,
      display: config.display
    }
  });
});

/**
 * Get today's overview and dashboard data
 */
app.get('/api/today', async (req, res) => {
  try {
    const todayDir = path.join(BASE_DIR, config.today.directory);
    const files = await fs.readdir(todayDir);

    const mdFiles = files.filter(f => f.endsWith('.md'));
    const result = {};

    for (const file of mdFiles) {
      const filePath = path.join(todayDir, file);
      const content = await fs.readFile(filePath, 'utf8');
      const parsed = matter(content);

      // Generate key from filename (00-overview.md -> overview)
      const key = file.replace(/^\d+-/, '').replace('.md', '').replace(/-/g, '_');

      result[key] = {
        filename: file,
        frontmatter: parsed.data,
        content: parsed.content,
        html: marked(parsed.content)
      };
    }

    res.json({
      success: true,
      directory: config.today.directory,
      files: result
    });
  } catch (error) {
    res.status(500).json({ success: false, error: error.message });
  }
});

/**
 * Get a specific file from _today
 */
app.get('/api/today/:file', async (req, res) => {
  try {
    const todayDir = path.join(BASE_DIR, config.today.directory);
    const files = await fs.readdir(todayDir);

    // Find file that matches (with or without number prefix)
    // Also check config mappings for week files
    const requestedFile = req.params.file;
    let targetFile = files.find(f =>
      f === requestedFile ||
      f.endsWith(`-${requestedFile}.md`) ||
      f === `${requestedFile}.md`
    );

    // Check week files mapping if not found
    if (!targetFile && requestedFile.startsWith('week-')) {
      const weekKey = requestedFile.replace('week-', '');
      const weekFile = config.today?.weekFiles?.[weekKey];
      if (weekFile) {
        targetFile = files.find(f => f === weekFile);
      }
    }

    if (!targetFile) {
      return res.status(404).json({ success: false, error: 'File not found' });
    }

    const filePath = path.join(todayDir, targetFile);
    const content = await fs.readFile(filePath, 'utf8');
    const parsed = matter(content);

    res.json({
      success: true,
      filename: targetFile,
      frontmatter: parsed.data,
      content: parsed.content,
      html: marked(parsed.content)
    });
  } catch (error) {
    res.status(500).json({ success: false, error: error.message });
  }
});

/**
 * Helper to get index data from a directory
 */
async function getItemIndex(itemPath, indexFile = '00-Index.md') {
  try {
    const indexPath = path.join(itemPath, indexFile);
    const content = await fs.readFile(indexPath, 'utf8');
    const parsed = matter(content);
    return {
      hasIndex: true,
      frontmatter: parsed.data,
      summary: extractFirstParagraph(parsed.content)
    };
  } catch {
    return { hasIndex: false, frontmatter: {}, summary: '' };
  }
}

/**
 * Extract first paragraph from markdown
 */
function extractFirstParagraph(content) {
  const lines = content.split('\n');
  let para = '';
  let inPara = false;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) {
      if (inPara) break;
      continue;
    }
    if (trimmed.startsWith('#') || trimmed.startsWith('-') || trimmed.startsWith('|')) {
      if (inPara) break;
      continue;
    }
    para += (para ? ' ' : '') + trimmed;
    inPara = true;
  }

  return para.slice(0, 200) + (para.length > 200 ? '...' : '');
}

/**
 * Extract date from filename like 2026-01-29-meeting.md
 */
function extractDateFromFilename(filename) {
  const match = filename.match(/^(\d{4}-\d{2}-\d{2})/);
  return match ? match[1] : null;
}

/**
 * Check if a directory has multi-BU structure
 */
async function isMultiBUDirectory(dirPath, indexFile) {
  try {
    const entries = await fs.readdir(dirPath, { withFileTypes: true });
    const subDirs = entries.filter(e => e.isDirectory() && !e.name.startsWith('.') && !e.name.startsWith('_'));

    if (subDirs.length === 0) return false;

    // Check if first subdir has the index file
    await fs.access(path.join(dirPath, subDirs[0].name, indexFile));
    return true;
  } catch {
    return false;
  }
}

/**
 * Generate dynamic section endpoints from config
 * NOTE: Order matters! More specific routes must come before wildcard routes
 */

// First, register folder routes (most specific)
config.sections.forEach(section => {
  const sectionDir = path.join(BASE_DIR, section.directory);

  app.get(`/api/${section.id}/:item(*)/folder/:folder`, async (req, res) => {
    try {
      const folderPath = path.join(sectionDir, req.params.item, req.params.folder);

      try {
        await fs.access(folderPath);
      } catch {
        return res.status(404).json({ success: false, error: 'Folder not found' });
      }

      const entries = await fs.readdir(folderPath, { withFileTypes: true });

      const files = [];
      for (const entry of entries) {
        if (entry.isFile() && entry.name.endsWith('.md')) {
          const filePath = path.join(folderPath, entry.name);
          const content = await fs.readFile(filePath, 'utf8');
          const parsed = matter(content);

          // Handle date - could be a Date object or string
          let fileDate = parsed.data.date || extractDateFromFilename(entry.name);
          if (fileDate instanceof Date) {
            fileDate = fileDate.toISOString().slice(0, 10);
          }

          files.push({
            name: entry.name,
            frontmatter: parsed.data,
            summary: extractFirstParagraph(parsed.content),
            date: fileDate
          });
        }
      }

      // Sort by date descending
      files.sort((a, b) => {
        const dateA = a.date || '';
        const dateB = b.date || '';
        return String(dateB).localeCompare(String(dateA));
      });

      res.json({
        success: true,
        section: section.id,
        item: req.params.item,
        folder: req.params.folder,
        files
      });
    } catch (error) {
      res.status(500).json({ success: false, error: error.message });
    }
  });
});

// Then register list and item routes
config.sections.forEach(section => {
  const sectionDir = path.join(BASE_DIR, section.directory);
  const indexFile = section.indexFile || '00-Index.md';

  /**
   * List all items in this section
   */
  app.get(`/api/${section.id}`, async (req, res) => {
    try {
      const entries = await fs.readdir(sectionDir, { withFileTypes: true });
      const items = [];

      for (const entry of entries) {
        if (entry.isDirectory() && !entry.name.startsWith('.') && !entry.name.startsWith('_')) {
          const itemPath = path.join(sectionDir, entry.name);

          // Check for multi-BU structure if supported
          if (section.structure === 'multi-bu' && config.features.multiBusinessUnit) {
            const isMultiBU = await isMultiBUDirectory(itemPath, indexFile);

            if (isMultiBU) {
              // Add each BU as a separate item
              const subEntries = await fs.readdir(itemPath, { withFileTypes: true });
              for (const subEntry of subEntries) {
                if (subEntry.isDirectory() && !subEntry.name.startsWith('.') && !subEntry.name.startsWith('_')) {
                  const buPath = path.join(itemPath, subEntry.name);
                  const indexData = await getItemIndex(buPath, indexFile);
                  items.push({
                    name: `${entry.name} / ${subEntry.name}`,
                    path: `${entry.name}/${subEntry.name}`,
                    isMultiBU: true,
                    parent: entry.name,
                    ...indexData
                  });
                }
              }
              continue;
            }
          }

          // Single item
          const indexData = await getItemIndex(itemPath, indexFile);
          items.push({
            name: entry.name,
            path: entry.name,
            isMultiBU: false,
            ...indexData
          });
        }
      }

      // Sort alphabetically
      items.sort((a, b) => a.name.localeCompare(b.name));

      res.json({
        success: true,
        section: section.id,
        count: items.length,
        items
      });
    } catch (error) {
      res.status(500).json({ success: false, error: error.message });
    }
  });

  /**
   * Get specific item details
   */
  app.get(`/api/${section.id}/:item(*)`, async (req, res) => {
    try {
      // Handle paths with slashes (for multi-BU)
      const itemPath = path.join(sectionDir, req.params.item);

      try {
        await fs.access(itemPath);
      } catch {
        return res.status(404).json({ success: false, error: 'Item not found' });
      }

      const entries = await fs.readdir(itemPath, { withFileTypes: true });

      // Get folders
      const folders = entries
        .filter(e => e.isDirectory() && !e.name.startsWith('.') && !e.name.startsWith('_'))
        .map(e => ({
          name: e.name,
          type: 'folder'
        }))
        .sort((a, b) => a.name.localeCompare(b.name));

      // Get files
      const files = entries
        .filter(e => e.isFile() && e.name.endsWith('.md'))
        .map(e => ({
          name: e.name,
          type: 'file'
        }))
        .sort((a, b) => a.name.localeCompare(b.name));

      // Get index if exists
      let index = null;
      try {
        const indexPath = path.join(itemPath, indexFile);
        const content = await fs.readFile(indexPath, 'utf8');
        const parsed = matter(content);
        index = {
          frontmatter: parsed.data,
          content: parsed.content,
          html: marked(parsed.content)
        };
      } catch {
        // No index file
      }

      res.json({
        success: true,
        section: section.id,
        item: req.params.item,
        index,
        folders,
        files
      });
    } catch (error) {
      res.status(500).json({ success: false, error: error.message });
    }
  });
});

/**
 * Read any file by path
 */
app.get('/api/file', async (req, res) => {
  try {
    const filePath = req.query.path;
    if (!filePath) {
      return res.status(400).json({ success: false, error: 'Path required' });
    }

    // Security: ensure path is within workspace directory
    const fullPath = path.resolve(BASE_DIR, filePath);
    if (!fullPath.startsWith(BASE_DIR)) {
      return res.status(403).json({ success: false, error: 'Access denied' });
    }

    const content = await fs.readFile(fullPath, 'utf8');
    const parsed = matter(content);

    res.json({
      success: true,
      path: filePath,
      frontmatter: parsed.data,
      content: parsed.content,
      html: marked(parsed.content)
    });
  } catch (error) {
    res.status(500).json({ success: false, error: error.message });
  }
});

/**
 * Search across files
 */
app.get('/api/search', async (req, res) => {
  try {
    const query = (req.query.q || '').toLowerCase();
    if (!query || query.length < 2) {
      return res.json({ success: true, results: [] });
    }

    const results = [];

    // Search in _today
    await searchDirectory(path.join(BASE_DIR, config.today.directory), config.today.directory, query, results);

    // Search in each section
    for (const section of config.sections) {
      const sectionDir = path.join(BASE_DIR, section.directory);
      try {
        const entries = await fs.readdir(sectionDir, { withFileTypes: true });
        for (const entry of entries.slice(0, 10)) { // Limit for performance
          if (entry.isDirectory() && !entry.name.startsWith('.')) {
            await searchDirectory(
              path.join(sectionDir, entry.name),
              `${section.directory}/${entry.name}`,
              query,
              results,
              2 // max depth
            );
          }
        }
      } catch {
        // Section directory might not exist
      }
    }

    // Sort by relevance (filename match first, then content match)
    results.sort((a, b) => {
      if (a.nameMatch && !b.nameMatch) return -1;
      if (!a.nameMatch && b.nameMatch) return 1;
      return 0;
    });

    res.json({
      success: true,
      query,
      count: results.length,
      results: results.slice(0, 50) // Limit results
    });
  } catch (error) {
    res.status(500).json({ success: false, error: error.message });
  }
});

/**
 * Recursively search a directory
 */
async function searchDirectory(dirPath, relativePath, query, results, maxDepth = 3, currentDepth = 0) {
  if (currentDepth > maxDepth) return;

  try {
    const entries = await fs.readdir(dirPath, { withFileTypes: true });

    for (const entry of entries) {
      if (entry.name.startsWith('.') || entry.name.startsWith('_')) continue;

      const entryPath = path.join(dirPath, entry.name);
      const entryRelative = `${relativePath}/${entry.name}`;

      if (entry.isDirectory()) {
        await searchDirectory(entryPath, entryRelative, query, results, maxDepth, currentDepth + 1);
      } else if (entry.isFile() && entry.name.endsWith('.md')) {
        const nameMatch = entry.name.toLowerCase().includes(query);

        try {
          const content = await fs.readFile(entryPath, 'utf8');
          const contentMatch = content.toLowerCase().includes(query);

          if (nameMatch || contentMatch) {
            const parsed = matter(content);

            // Find matching line for context
            let matchContext = '';
            if (contentMatch) {
              const lines = content.split('\n');
              for (const line of lines) {
                if (line.toLowerCase().includes(query)) {
                  matchContext = line.trim().slice(0, 100);
                  break;
                }
              }
            }

            results.push({
              name: entry.name,
              path: entryRelative,
              nameMatch,
              contentMatch,
              matchContext,
              frontmatter: parsed.data
            });
          }
        } catch {
          // Skip files that can't be read
        }
      }
    }
  } catch {
    // Skip directories that can't be read
  }
}

// SPA fallback - serve index.html for all non-API routes
app.get('*', (req, res) => {
  res.sendFile(path.join(__dirname, 'public', 'index.html'));
});

// Start server
app.listen(PORT, () => {
  console.log(`
  ╔═══════════════════════════════════════════════════╗
  ║                                                   ║
  ║   DailyOS UI Server                               ║
  ║                                                   ║
  ║   Local:     http://localhost:${PORT}                 ║
  ║   Workspace: ${config.workspace.name.padEnd(35)}║
  ║   Role:      ${(config.workspace.role || 'custom').padEnd(35)}║
  ║   Base:      ${BASE_DIR.slice(-35).padStart(35)}║
  ║                                                   ║
  ╚═══════════════════════════════════════════════════╝
  `);
});

module.exports = app;
