/**
 * Setup API routes for the wizard.
 *
 * Handles step execution, progress tracking, and rollback.
 */

import { Router } from 'express';
import { spawn } from 'child_process';
import path from 'path';
import { fileURLToPath } from 'url';
import {
  createSession,
  getSession,
  updateSession,
  completeStep,
  addError,
  persistSession,
  getProgress,
} from '../state.js';

const router = Router();

// Track active SSE connections by session
const sseConnections = new Map();

// Get project root (server's parent directory)
const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = path.resolve(__dirname, '../..');

/**
 * POST /api/setup/start
 * Create a new setup session
 */
router.post('/start', (req, res) => {
  const session = createSession();
  res.json({
    success: true,
    session: {
      id: session.id,
      currentStep: session.currentStep,
    }
  });
});

/**
 * GET /api/setup/status/:sessionId
 * Get current session status
 */
router.get('/status/:sessionId', (req, res) => {
  const session = getSession(req.params.sessionId);

  if (!session) {
    return res.status(404).json({
      success: false,
      error: 'Session not found',
    });
  }

  res.json({
    success: true,
    session: {
      id: session.id,
      currentStep: session.currentStep,
      completedSteps: session.completedSteps,
      config: session.config,
      progress: getProgress(session.id),
    }
  });
});

/**
 * POST /api/setup/step/:stepId
 * Execute a setup step
 */
router.post('/step/:stepId', async (req, res) => {
  const { stepId } = req.params;
  const { sessionId, config } = req.body;

  const session = getSession(sessionId);
  if (!session) {
    return res.status(404).json({
      success: false,
      error: 'Session not found',
    });
  }

  // Update session config with any new values
  if (config) {
    updateSession(sessionId, { config });
  }

  try {
    // Execute the Python step runner
    const result = await executeStep(stepId, session, sseConnections.get(sessionId));

    if (result.success) {
      completeStep(sessionId, stepId, result.rollbackData);

      // Persist state after workspace is set
      if (session.config.workspacePath) {
        await persistSession(sessionId);
      }
    } else {
      addError(sessionId, {
        step: stepId,
        message: result.error,
      });
    }

    res.json({
      success: result.success,
      result: result.result,
      error: result.error,
      progress: getProgress(sessionId),
    });

  } catch (err) {
    addError(sessionId, {
      step: stepId,
      message: err.message,
    });

    res.status(500).json({
      success: false,
      error: err.message,
    });
  }
});

/**
 * GET /api/setup/stream/:sessionId
 * Server-Sent Events stream for real-time progress
 */
router.get('/stream/:sessionId', (req, res) => {
  const { sessionId } = req.params;

  const session = getSession(sessionId);
  if (!session) {
    return res.status(404).json({
      success: false,
      error: 'Session not found',
    });
  }

  // Set up SSE headers
  res.setHeader('Content-Type', 'text/event-stream');
  res.setHeader('Cache-Control', 'no-cache');
  res.setHeader('Connection', 'keep-alive');
  res.setHeader('Access-Control-Allow-Origin', '*');

  // Send initial connection event
  res.write(`event: connected\ndata: ${JSON.stringify({ sessionId })}\n\n`);

  // Store connection for this session
  sseConnections.set(sessionId, res);

  // Clean up on close
  req.on('close', () => {
    sseConnections.delete(sessionId);
  });
});

/**
 * POST /api/setup/rollback
 * Rollback the last completed step
 */
router.post('/rollback', async (req, res) => {
  const { sessionId } = req.body;

  const session = getSession(sessionId);
  if (!session) {
    return res.status(404).json({
      success: false,
      error: 'Session not found',
    });
  }

  const lastRollback = session.rollbackStack.pop();
  if (!lastRollback) {
    return res.json({
      success: true,
      message: 'Nothing to rollback',
    });
  }

  try {
    // Execute rollback via Python
    const result = await executeRollback(lastRollback.stepId, lastRollback.data);

    // Remove step from completed
    const stepIndex = session.completedSteps.indexOf(lastRollback.stepId);
    if (stepIndex > -1) {
      session.completedSteps.splice(stepIndex, 1);
    }

    // Update current step
    session.currentStep = Math.max(0, session.currentStep - 1);

    res.json({
      success: true,
      rolledBack: lastRollback.stepId,
      progress: getProgress(sessionId),
    });

  } catch (err) {
    res.status(500).json({
      success: false,
      error: `Rollback failed: ${err.message}`,
    });
  }
});

/**
 * POST /api/setup/complete
 * Mark setup as complete
 */
router.post('/complete', async (req, res) => {
  const { sessionId } = req.body;

  const session = getSession(sessionId);
  if (!session) {
    return res.status(404).json({
      success: false,
      error: 'Session not found',
    });
  }

  completeStep(sessionId, 'complete');

  // Final persist
  if (session.config.workspacePath) {
    await persistSession(sessionId);
  }

  res.json({
    success: true,
    message: 'Setup complete!',
    workspacePath: session.config.workspacePath,
  });
});

/**
 * Execute a Python step
 */
async function executeStep(stepId, session, sseResponse) {
  return new Promise((resolve, reject) => {
    const runnerPath = path.join(PROJECT_ROOT, 'src', 'setup', 'runner.py');

    const input = JSON.stringify({
      step: stepId,
      config: session.config,
    });

    const python = spawn('python3', [runnerPath], {
      cwd: PROJECT_ROOT,
      env: {
        ...process.env,
        PYTHONPATH: path.join(PROJECT_ROOT, 'src'),
      },
    });

    let stdout = '';
    let stderr = '';

    python.stdout.on('data', (data) => {
      const text = data.toString();

      // Check for progress updates (PROGRESS:JSON format)
      const lines = text.split('\n');
      for (const line of lines) {
        if (line.startsWith('PROGRESS:')) {
          const progressJson = line.slice(9);
          try {
            const progress = JSON.parse(progressJson);
            // Send SSE update if connected
            if (sseResponse) {
              sseResponse.write(`event: progress\ndata: ${progressJson}\n\n`);
            }
          } catch (e) {
            // Ignore malformed progress
          }
        } else if (line.trim()) {
          stdout += line + '\n';
        }
      }
    });

    python.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    python.on('close', (code) => {
      if (code !== 0) {
        resolve({
          success: false,
          error: stderr || `Step ${stepId} failed with code ${code}`,
        });
        return;
      }

      try {
        // Find the last JSON object in stdout (trim to handle trailing newlines)
        const trimmedOutput = stdout.trim();
        const jsonMatch = trimmedOutput.match(/\{[\s\S]*\}$/);
        if (jsonMatch) {
          const result = JSON.parse(jsonMatch[0]);
          resolve({
            success: result.success !== false,
            result: result.result,
            rollbackData: result.rollbackData,
            error: result.error,
          });
        } else {
          console.error('No JSON match found in stdout:', trimmedOutput);
          resolve({
            success: true,
            result: { output: stdout },
          });
        }
      } catch (e) {
        console.error('JSON parse error:', e.message, 'stdout:', stdout);
        resolve({
          success: true,
          result: { output: stdout },
        });
      }
    });

    python.on('error', (err) => {
      reject(err);
    });

    // Send input
    python.stdin.write(input);
    python.stdin.end();
  });
}

/**
 * POST /api/setup/google/upload-credentials
 * Upload and validate Google OAuth credentials
 */
router.post('/google/upload-credentials', async (req, res) => {
  const { content } = req.body;

  if (!content) {
    return res.status(400).json({
      success: false,
      error: 'No credentials content provided',
    });
  }

  try {
    // Validate and save via Python
    const result = await validateAndSaveGoogleCredentials(content);
    res.json(result);
  } catch (err) {
    res.status(500).json({
      success: false,
      error: err.message,
    });
  }
});

/**
 * POST /api/setup/google/test-auth
 * Test Google API authentication
 */
router.post('/google/test-auth', async (req, res) => {
  try {
    const result = await testGoogleAuth();
    res.json(result);
  } catch (err) {
    res.status(500).json({
      success: false,
      error: err.message,
    });
  }
});

/**
 * GET /api/setup/google/status
 * Get current Google API setup status
 */
router.get('/google/status', async (req, res) => {
  try {
    const result = await getGoogleSetupStatus();
    res.json(result);
  } catch (err) {
    res.status(500).json({
      success: false,
      error: err.message,
    });
  }
});

/**
 * Validate and save Google credentials via Python
 */
async function validateAndSaveGoogleCredentials(content) {
  return new Promise((resolve, reject) => {
    const pythonCode = `
import sys
import json
sys.path.insert(0, '${PROJECT_ROOT}/src')
from steps.google_api import validate_credentials_json, save_credentials_secure

content = '''${content.replace(/'/g, "\\'")}'''
is_valid, error = validate_credentials_json(content)

if not is_valid:
    print(json.dumps({"success": False, "error": error}))
else:
    success, save_error = save_credentials_secure(content)
    if success:
        print(json.dumps({"success": True, "message": "Credentials saved to ~/.dailyos/google/"}))
    else:
        print(json.dumps({"success": False, "error": save_error}))
`;

    const python = spawn('python3', ['-c', pythonCode], {
      cwd: PROJECT_ROOT,
      env: {
        ...process.env,
        PYTHONPATH: path.join(PROJECT_ROOT, 'src'),
      },
    });

    let stdout = '';
    let stderr = '';

    python.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    python.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    python.on('close', (code) => {
      if (code !== 0) {
        resolve({
          success: false,
          error: stderr || `Validation failed with code ${code}`,
        });
        return;
      }

      try {
        const result = JSON.parse(stdout.trim());
        resolve(result);
      } catch (e) {
        resolve({
          success: false,
          error: 'Failed to parse validation result',
        });
      }
    });

    python.on('error', reject);
  });
}

/**
 * Test Google authentication via Python
 */
async function testGoogleAuth() {
  return new Promise((resolve, reject) => {
    const scriptPath = path.join(PROJECT_ROOT, 'templates', 'scripts', 'google', 'google_api.py');

    const python = spawn('python3', [scriptPath, 'auth'], {
      cwd: PROJECT_ROOT,
      env: {
        ...process.env,
        PYTHONPATH: path.join(PROJECT_ROOT, 'src'),
      },
    });

    let stdout = '';
    let stderr = '';

    python.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    python.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    python.on('close', (code) => {
      if (code === 0) {
        resolve({
          success: true,
          message: 'Authentication successful',
        });
      } else {
        resolve({
          success: false,
          error: stderr || 'Authentication failed',
        });
      }
    });

    python.on('error', reject);
  });
}

/**
 * Get Google setup status via Python
 */
async function getGoogleSetupStatus() {
  return new Promise((resolve, reject) => {
    const pythonCode = `
import sys
import json
sys.path.insert(0, '${PROJECT_ROOT}/src')
from steps.google_api import verify_google_setup

status = verify_google_setup()
print(json.dumps({"success": True, "status": status}))
`;

    const python = spawn('python3', ['-c', pythonCode], {
      cwd: PROJECT_ROOT,
      env: {
        ...process.env,
        PYTHONPATH: path.join(PROJECT_ROOT, 'src'),
      },
    });

    let stdout = '';
    let stderr = '';

    python.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    python.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    python.on('close', (code) => {
      if (code !== 0) {
        resolve({
          success: false,
          error: stderr || 'Failed to get status',
        });
        return;
      }

      try {
        const result = JSON.parse(stdout.trim());
        resolve(result);
      } catch (e) {
        resolve({
          success: false,
          error: 'Failed to parse status result',
        });
      }
    });

    python.on('error', reject);
  });
}

/**
 * Execute a rollback operation
 */
async function executeRollback(stepId, rollbackData) {
  return new Promise((resolve, reject) => {
    const runnerPath = path.join(PROJECT_ROOT, 'src', 'setup', 'runner.py');

    const input = JSON.stringify({
      rollback: true,
      step: stepId,
      rollbackData,
    });

    const python = spawn('python3', [runnerPath], {
      cwd: PROJECT_ROOT,
      env: {
        ...process.env,
        PYTHONPATH: path.join(PROJECT_ROOT, 'src'),
      },
    });

    let stdout = '';
    let stderr = '';

    python.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    python.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    python.on('close', (code) => {
      if (code !== 0) {
        reject(new Error(stderr || `Rollback failed with code ${code}`));
        return;
      }

      try {
        const result = JSON.parse(stdout);
        resolve(result);
      } catch (e) {
        resolve({ success: true });
      }
    });

    python.on('error', reject);

    python.stdin.write(input);
    python.stdin.end();
  });
}

export default router;
