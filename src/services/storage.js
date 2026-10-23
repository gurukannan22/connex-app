// ============================================================
// services/storage.js — Local connection history manager
// Manages recent 6-digit device IDs with deduplication & validation
// ============================================================

export const RECENT_KEY = "connex.recent";
export const MAX_RECENT_ITEMS = 6;

/**
 * Validate if a string is a valid 6-digit device ID
 * @param {string} id
 * @returns {boolean}
 */
export function isValidDeviceId(id) {
  return typeof id === "string" && /^\d{6}$/.test(id.trim());
}

/**
 * Retrieve recent device IDs from storage
 * @param {Storage} [storage=localStorage]
 * @returns {string[]}
 */
export function getRecent(storage = typeof localStorage !== "undefined" ? localStorage : null) {
  if (!storage) return [];
  try {
    const raw = storage.getItem(RECENT_KEY);
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter(isValidDeviceId) : [];
  } catch {
    return [];
  }
}

/**
 * Add a device ID to recent list, deduplicating and limiting to MAX_RECENT_ITEMS
 * @param {string} id
 * @param {Storage} [storage=localStorage]
 * @returns {string[]} updated list
 */
export function saveRecent(id, storage = typeof localStorage !== "undefined" ? localStorage : null) {
  const trimmed = typeof id === "string" ? id.trim() : "";
  if (!isValidDeviceId(trimmed)) return getRecent(storage);

  const existing = getRecent(storage).filter((x) => x !== trimmed);
  const updated = [trimmed, ...existing].slice(0, MAX_RECENT_ITEMS);

  if (storage) {
    try {
      storage.setItem(RECENT_KEY, JSON.stringify(updated));
    } catch {
      // Storage quota or sandbox restrictions
    }
  }

  return updated;
}

/**
 * Clear all recent device IDs
 * @param {Storage} [storage=localStorage]
 */
export function clearRecent(storage = typeof localStorage !== "undefined" ? localStorage : null) {
  if (storage) {
    try {
      storage.removeItem(RECENT_KEY);
    } catch {}
  }
}
