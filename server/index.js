/**
 * Daily Operating System - Setup Wizard Server
 *
 * Express server that serves the setup wizard UI and handles
 * setup step execution via Python subprocess calls.
 *
 * Usage:
 *   npm start        # Start on port 5050
 *   npm run dev      # Start with file watching
 */

import express from 'express';
import path from 'path';
import { fileURLToPath } from 'url';
import setupRoutes from './routes/setup.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = path.resolve(__dirname, '..');

const app = express();
const PORT = process.env.PORT || 5050;

// Middleware
app.use(express.json());

// CORS for development
app.use((req, res, next) => {
  res.header('Access-Control-Allow-Origin', '*');
  res.header('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.header('Access-Control-Allow-Headers', 'Content-Type');
  if (req.method === 'OPTIONS') {
    return res.sendStatus(200);
  }
  next();
});

// API routes
app.use('/api/setup', setupRoutes);

// Serve static files from docs directory
app.use(express.static(path.join(PROJECT_ROOT, 'docs')));

// Serve wizard as main page
app.get('/', (req, res) => {
  res.sendFile(path.join(PROJECT_ROOT, 'docs', 'index.html'));
});

// Health check
app.get('/api/health', (req, res) => {
  res.json({
    status: 'ok',
    version: '1.0.0',
    timestamp: new Date().toISOString(),
  });
});

// Error handling
app.use((err, req, res, next) => {
  console.error('Server error:', err);
  res.status(500).json({
    success: false,
    error: 'Internal server error',
    message: err.message,
  });
});

// Start server
app.listen(PORT, () => {
  console.log('');
  console.log('  Daily Operating System - Setup Wizard');
  console.log('  =====================================');
  console.log('');
  console.log(`  Server running at: http://localhost:${PORT}`);
  console.log('');
  console.log('  API endpoints:');
  console.log('    POST /api/setup/start        - Create new session');
  console.log('    GET  /api/setup/status/:id   - Get session status');
  console.log('    POST /api/setup/step/:stepId - Execute a step');
  console.log('    GET  /api/setup/stream/:id   - SSE progress stream');
  console.log('    POST /api/setup/rollback     - Undo last step');
  console.log('    POST /api/setup/complete     - Mark setup done');
  console.log('');
  console.log('  Press Ctrl+C to stop');
  console.log('');
});
