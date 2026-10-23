// ============================================================
// screens/remoteScreen.js — Remote desktop viewer screen markup
// ============================================================

import { icon } from "../components/icons.js";

export function renderRemoteScreen() {
  return `
    <div id="screen-remote" class="screen">
      <div class="remote-shell">
        <div class="remote-toolbar">
          <span class="live"><span class="dot"></span>Connected</span>
          <button class="btn btn-sm" id="btn-fullscreen">${icon("maximize", "icon icon-sm")} Fullscreen</button>
          <button class="btn btn-danger btn-sm" id="btn-disconnect">${icon("x", "icon icon-sm")} Disconnect</button>
        </div>
        <div class="remote-canvas-wrap"><canvas id="remote-canvas"></canvas></div>
        <div class="custom-cursor" id="custom-cursor" style="opacity:0"></div>
      </div>
    </div>`;
}
