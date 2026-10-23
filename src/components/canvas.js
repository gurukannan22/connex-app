// ============================================================
// components/canvas.js — Remote display canvas renderer & input
// Handles high-fps frame rendering and normalized input mapping
// ============================================================

import { injectInput } from "../services/tauri.js";

let currentCanvasWidth = 0;
let currentCanvasHeight = 0;
let remoteInteractionsAttached = false;

/**
 * Normalize client pointer coordinates relative to bounding box
 * @param {number} clientX
 * @param {number} clientY
 * @param {{ left: number, top: number, width: number, height: number }} rect
 * @returns {{ x: number, y: number } | null}
 */
export function normalizeCoordinates(clientX, clientY, rect) {
  if (!rect || rect.width <= 0 || rect.height <= 0) return null;
  const rawX = (clientX - rect.left) / rect.width;
  const rawY = (clientY - rect.top) / rect.height;
  return {
    x: Math.max(0, Math.min(1, rawX)),
    y: Math.max(0, Math.min(1, rawY)),
  };
}

/**
 * Render a JPEG frame to the canvas, dynamically updating dimensions
 * @param {HTMLCanvasElement} canvas
 * @param {string} b64
 * @param {number} width
 * @param {number} height
 */
export function renderFrame(canvas, b64, width, height) {
  if (!canvas) return;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const img = new Image();
  img.onload = () => {
    const targetW = width > 0 ? width : img.naturalWidth;
    const targetH = height > 0 ? height : img.naturalHeight;
    if (targetW > 0 && targetH > 0 && (currentCanvasWidth !== targetW || currentCanvasHeight !== targetH)) {
      canvas.width = targetW;
      canvas.height = targetH;
      currentCanvasWidth = targetW;
      currentCanvasHeight = targetH;
    }
    ctx.drawImage(img, 0, 0);
  };
  img.src = `data:image/jpeg;base64,${b64}`;
}

/**
 * Attach mouse, wheel, and keyboard listeners to canvas and window
 * @param {HTMLCanvasElement} canvas
 * @param {HTMLElement} cursor
 * @param {() => boolean} isConnected
 */
export function setupCanvasInteractions(canvas, cursor, isConnected) {
  if (remoteInteractionsAttached || !canvas) return;
  remoteInteractionsAttached = true;

  canvas.addEventListener("mousemove", (e) => {
    if (!isConnected()) return;
    const rect = canvas.getBoundingClientRect();
    const coords = normalizeCoordinates(e.clientX, e.clientY, rect);
    if (!coords) return;

    if (cursor) {
      cursor.style.left = `${e.clientX}px`;
      cursor.style.top = `${e.clientY}px`;
    }

    injectInput({ type: "mouse_move", x: coords.x, y: coords.y }).catch(() => {});
  });

  const buttonMap = { 0: "left", 1: "middle", 2: "right" };
  ["mousedown", "mouseup"].forEach((eventName) => {
    canvas.addEventListener(eventName, (e) => {
      if (!isConnected()) return;
      e.preventDefault();
      injectInput({
        type: "mouse_button",
        button: buttonMap[e.button] ?? "left",
        pressed: eventName === "mousedown",
      }).catch(() => {});
    });
  });

  canvas.addEventListener(
    "wheel",
    (e) => {
      if (!isConnected()) return;
      e.preventDefault();
      injectInput({
        type: "mouse_scroll",
        delta_x: Math.round(e.deltaX / 10),
        delta_y: Math.round(e.deltaY / 10),
      }).catch(() => {});
    },
    { passive: false }
  );

  window.addEventListener("keydown", (e) => {
    if (!isConnected()) return;
    if (e.key === "F11" || ((e.metaKey || e.ctrlKey) && (e.key === "q" || e.key === "w"))) return;
    e.preventDefault();
    injectInput({ type: "key_press", key: e.key, pressed: true }).catch(() => {});
  });

  window.addEventListener("keyup", (e) => {
    if (!isConnected()) return;
    injectInput({ type: "key_press", key: e.key, pressed: false }).catch(() => {});
  });

  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
  canvas.addEventListener("mouseleave", () => {
    if (cursor) cursor.style.opacity = "0";
  });
  canvas.addEventListener("mouseenter", () => {
    if (cursor && isConnected()) cursor.style.opacity = "1";
  });
}
