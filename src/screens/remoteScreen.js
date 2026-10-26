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
        <aside class="chat-panel" aria-label="Session chat">
          <div class="chat-heading">Session chat <span>Messages are sent directly to this peer.</span></div>
          <div class="chat-messages" aria-live="polite"></div>
          <form class="chat-compose chat-form">
            <input class="input chat-input" maxlength="2000" placeholder="Write a message…" autocomplete="off" />
            <button class="btn btn-sm chat-file-button" type="button" title="Send a file up to 2 MB">File</button>
            <button class="btn btn-primary btn-sm" type="submit">Send</button>
          </form>
          <input class="chat-file-input" type="file" hidden />
        </aside>
      </div>
    </div>`;
}
