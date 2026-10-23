// ============================================================
// components/toast.js — Toast notification manager
// Non-intrusive feedback for actions, warnings, and errors
// ============================================================

import { icon } from "./icons.js";

/**
 * Display a temporary floating toast notification
 * @param {string} message
 * @param {"info" | "success" | "error" | "warning"} [type="info"]
 * @param {number} [duration=3600]
 */
export function showToast(message, type = "info", duration = 3600) {
  const container = document.getElementById("toast-container");
  if (!container) return;

  const iconName = { info: "info", success: "check", error: "alert", warning: "alert" }[type] ?? "info";
  const el = document.createElement("div");
  el.className = `toast ${type}`;
  el.innerHTML = `${icon(iconName)}<span>${message}</span>`;
  container.appendChild(el);

  setTimeout(() => {
    el.style.animation = "toastIn 0.2s ease reverse";
    setTimeout(() => el.remove(), 200);
  }, duration);
}
