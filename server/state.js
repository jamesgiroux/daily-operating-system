/**
 * Session state management for the setup wizard.
 *
 * Maintains setup progress in memory with file backup for recovery.
 */

import { writeFile, readFile } from 'fs/promises';
import { existsSync } from 'fs';
import path from 'path';

// In-memory session store
const sessions = new Map();

// State file path (relative to workspace once set)
const STATE_FILENAME = '.setup-state.json';

/**
 * Create a new session
 * @returns {Object} Session object with id and initial state
 */
export function createSession() {
  const id = crypto.randomUUID();
  const session = {
    id,
    createdAt: new Date().toISOString(),
    currentStep: 0,
    completedSteps: [],
    config: {
      workspacePath: null,
      role: null,
      googleApiMode: null,
      claudeMdMode: null,
      skillsMode: null,
    },
    rollbackStack: [],
    errors: [],
  };

  sessions.set(id, session);
  return session;
}

/**
 * Get a session by ID
 * @param {string} sessionId
 * @returns {Object|null} Session object or null if not found
 */
export function getSession(sessionId) {
  return sessions.get(sessionId) || null;
}

/**
 * Update session state
 * @param {string} sessionId
 * @param {Object} updates - Partial updates to merge
 * @returns {Object} Updated session
 */
export function updateSession(sessionId, updates) {
  const session = sessions.get(sessionId);
  if (!session) {
    throw new Error(`Session not found: ${sessionId}`);
  }

  // Deep merge config if provided
  if (updates.config) {
    session.config = { ...session.config, ...updates.config };
    delete updates.config;
  }

  // Merge other updates
  Object.assign(session, updates);
  session.updatedAt = new Date().toISOString();

  return session;
}

/**
 * Mark a step as complete
 * @param {string} sessionId
 * @param {string} stepId
 * @param {Object} rollbackData - Data needed to undo this step
 */
export function completeStep(sessionId, stepId, rollbackData = null) {
  const session = sessions.get(sessionId);
  if (!session) {
    throw new Error(`Session not found: ${sessionId}`);
  }

  if (!session.completedSteps.includes(stepId)) {
    session.completedSteps.push(stepId);
  }

  if (rollbackData) {
    session.rollbackStack.push({
      stepId,
      data: rollbackData,
      timestamp: new Date().toISOString(),
    });
  }

  session.currentStep = Math.max(session.currentStep, getStepIndex(stepId) + 1);
  session.updatedAt = new Date().toISOString();

  return session;
}

/**
 * Get step index from step ID
 */
function getStepIndex(stepId) {
  const stepOrder = [
    'welcome',
    'prerequisites',
    'workspace',
    'role',
    'directories',
    'git',
    'google',
    'claudemd',
    'skills',
    'verification',
    'complete'
  ];
  return stepOrder.indexOf(stepId);
}

/**
 * Add error to session
 * @param {string} sessionId
 * @param {Object} error
 */
export function addError(sessionId, error) {
  const session = sessions.get(sessionId);
  if (!session) {
    throw new Error(`Session not found: ${sessionId}`);
  }

  session.errors.push({
    ...error,
    timestamp: new Date().toISOString(),
  });

  return session;
}

/**
 * Save session state to workspace file for recovery
 * @param {string} sessionId
 */
export async function persistSession(sessionId) {
  const session = sessions.get(sessionId);
  if (!session || !session.config.workspacePath) {
    return;
  }

  const stateFile = path.join(session.config.workspacePath, STATE_FILENAME);

  try {
    await writeFile(stateFile, JSON.stringify(session, null, 2));
  } catch (err) {
    console.error(`Failed to persist session state: ${err.message}`);
  }
}

/**
 * Load session state from workspace file
 * @param {string} workspacePath
 * @returns {Object|null} Loaded session or null
 */
export async function loadPersistedSession(workspacePath) {
  const stateFile = path.join(workspacePath, STATE_FILENAME);

  if (!existsSync(stateFile)) {
    return null;
  }

  try {
    const content = await readFile(stateFile, 'utf-8');
    const session = JSON.parse(content);

    // Re-register in memory
    sessions.set(session.id, session);

    return session;
  } catch (err) {
    console.error(`Failed to load persisted session: ${err.message}`);
    return null;
  }
}

/**
 * Delete session
 * @param {string} sessionId
 */
export function deleteSession(sessionId) {
  sessions.delete(sessionId);
}

/**
 * Get progress percentage
 * @param {string} sessionId
 * @returns {number} Progress percentage (0-100)
 */
export function getProgress(sessionId) {
  const session = sessions.get(sessionId);
  if (!session) return 0;

  const totalSteps = 11; // Including welcome and complete
  return Math.round((session.completedSteps.length / totalSteps) * 100);
}
