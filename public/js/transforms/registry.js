/**
 * DailyOS Transform Registry
 * Manages page-specific transforms with pattern matching
 *
 * @typedef {Object} Transform
 * @property {string} name - Unique identifier for the transform
 * @property {function(Element): boolean} detect - Returns true if transform should apply
 * @property {function(Element): void} apply - Applies the transformation to container
 */

const TransformRegistry = {
  /** @type {Transform[]} */
  transforms: [],

  /**
   * Register a transform handler
   * @param {Object} transform - Transform object with name, detect, and apply methods
   */
  register(transform) {
    if (!transform.name || !transform.detect || !transform.apply) {
      console.error('[DailyOS] Invalid transform:', transform);
      return;
    }
    this.transforms.push(transform);
  },

  /**
   * Apply the first matching transform to a container
   * @param {Element} container - DOM container to transform
   * @returns {boolean} True if a transform was applied
   */
  apply(container) {
    for (const transform of this.transforms) {
      try {
        if (transform.detect(container)) {
          transform.apply(container);
          return true;
        }
      } catch (err) {
        console.error(`[DailyOS] Transform "${transform.name}" failed:`, err);
      }
    }
    return false;
  },

  /**
   * Get registered transform names
   * @returns {string[]}
   */
  list() {
    return this.transforms.map(t => t.name);
  },

  /**
   * Clear all registered transforms
   */
  clear() {
    this.transforms = [];
  }
};

// Make available globally
window.TransformRegistry = TransformRegistry;
