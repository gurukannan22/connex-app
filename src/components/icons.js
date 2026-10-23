// ============================================================
// components/icons.js — Inline vector icon catalog
// Crisp vector paths at any DPI without emoji artifacts
// ============================================================

export const ICONS = {
  link: `<path d="M9 15l6-6"/><path d="M11 6l1-1a4 4 0 0 1 6 6l-1 1"/><path d="M13 18l-1 1a4 4 0 0 1-6-6l1-1"/>`,
  monitor: `<rect x="3" y="4" width="18" height="12" rx="2"/><path d="M8 20h8M12 16v4"/>`,
  pointer: `<path d="M5 3l15 7-6.5 2.2L11 19z"/>`,
  copy: `<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h8"/>`,
  refresh: `<path d="M20 11a8 8 0 1 0-1.5 5.5"/><path d="M20 4v5h-5"/>`,
  check: `<path d="M4 12l5 5L20 6"/>`,
  lock: `<rect x="4" y="10" width="16" height="10" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3"/>`,
  maximize: `<path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5"/>`,
  x: `<path d="M6 6l12 12M18 6L6 18"/>`,
  info: `<circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8h.01"/>`,
  alert: `<path d="M12 3l9 16H3z"/><path d="M12 10v4M12 17h.01"/>`,
  send: `<path d="M21 4L3 11l6 2 2 6z"/><path d="M21 4l-10 9"/>`,
  arrowRight: `<path d="M5 12h14M13 6l6 6-6 6"/>`,
  share: `<path d="M4 12v7a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-7"/><path d="M12 3v12M8 7l4-4 4 4"/>`,
  brand: `<path d="M5 10.2H14.3C15.6 10.2 16.4 9.4 16.4 8.1V5.9C16.4 4.6 15.6 3.8 14.3 3.8H8.2"/><path d="M19 13.8H9.7C8.4 13.8 7.6 14.6 7.6 15.9V18.1C7.6 19.4 8.4 20.2 9.7 20.2H15.8"/>`,
};

/**
 * Generate an SVG element string with standard viewBox and attributes
 * @param {keyof typeof ICONS} name
 * @param {string} [cls="icon"]
 * @returns {string}
 */
export function icon(name, cls = "icon") {
  const path = ICONS[name] ?? "";
  return `<svg class="${cls}" viewBox="0 0 24 24">${path}</svg>`;
}
