// ============================================================
// screens/homeScreen.js — Home setup screen markup & views
// ============================================================

import { icon } from "../components/icons.js";

export function renderHomeScreen() {
  return `
    <div id="screen-home" class="screen active">
      <div class="content">
        <div class="content-inner">
          <div class="page-head">
            <h1>Remote desktop</h1>
            <p>Share this computer, or connect to another.</p>
          </div>

          <div class="home-grid">

            <!-- This computer -->
            <section class="panel">
              <div class="panel-title"><span class="tile">${icon("monitor", "icon icon-sm")}</span><h2>This computer</h2></div>
              <p class="panel-sub">Share your ID and passcode with someone you trust.</p>

              <div class="creds">
                <div class="cred">
                  <div class="label">Device ID</div>
                  <div class="value loading" data-device-id title="Click to copy"></div>
                </div>
                <div class="cred">
                  <div class="label">Passcode</div>
                  <div class="value loading" data-passcode title="Click to copy"></div>
                </div>
              </div>

              <div class="inline-actions">
                <button class="btn btn-ghost btn-sm" id="btn-copy-identity">${icon("copy", "icon icon-sm")} Copy</button>
                <button class="btn btn-ghost btn-sm" id="btn-rotate-passcode">${icon("refresh", "icon icon-sm")} New passcode</button>
              </div>

              <button class="btn btn-primary btn-lg btn-block" id="btn-share-start">${icon("share", "icon icon-sm")} Start sharing</button>

              <div class="collapse" id="share-collapse">
                <div class="hr"></div>
                <div class="steps">
                  <div class="stp" id="share-step-1"><div class="stp-num">1</div><div class="stp-body"><div class="t">Send your payload</div><div class="d">Copy it and send to the other person.</div></div></div>
                  <div class="stp" id="share-step-2"><div class="stp-num">2</div><div class="stp-body"><div class="t">Wait for their reply</div><div class="d">They paste it and send a reply back.</div></div></div>
                  <div class="stp" id="share-step-3"><div class="stp-num">3</div><div class="stp-body"><div class="t">Accept the connection</div><div class="d">The passcode is checked before sharing.</div></div></div>
                </div>
                <div class="field">
                  <div class="label">Your payload</div>
                  <textarea class="input input-mono" id="host-payload-box" rows="3" readonly></textarea>
                  <button class="btn btn-ghost btn-sm" id="btn-host-copy-payload">${icon("copy", "icon icon-sm")} Copy payload</button>
                </div>
                <div class="field">
                  <div class="label">Paste their reply</div>
                  <textarea class="input input-mono" id="host-answer-input" rows="2" placeholder="Paste the reply…"></textarea>
                  <button class="btn btn-primary btn-sm" id="btn-host-apply-answer">Accept &amp; connect</button>
                </div>
                <span class="flow-status" id="share-status"><span class="dot"></span>Idle</span>
              </div>
            </section>

            <!-- Connect to a computer -->
            <section class="panel">
              <div class="panel-title"><span class="tile">${icon("pointer", "icon icon-sm")}</span><h2>Connect to a computer</h2></div>
              <p class="panel-sub">Enter their passcode and paste the payload they sent.</p>

              <div class="creds">
                <div class="field" style="flex:1">
                  <div class="label">Passcode</div>
                  <input class="input input-code" id="ctrl-passcode" inputmode="numeric" maxlength="6" placeholder="••••••" />
                </div>
                <div class="field" style="flex:1">
                  <div class="label">Device ID <span class="opt">(optional)</span></div>
                  <input class="input input-code" id="ctrl-device-id" inputmode="numeric" maxlength="6" placeholder="123456" />
                </div>
              </div>

              <div class="field">
                <div class="label">Payload</div>
                <textarea class="input input-mono" id="ctrl-payload-input" rows="3" placeholder="Paste the payload they sent…"></textarea>
              </div>

              <button class="btn btn-primary btn-lg btn-block" id="btn-controller-connect" disabled>Connect ${icon("arrowRight", "icon icon-sm")}</button>

              <div class="collapse" id="connect-collapse">
                <div class="hr"></div>
                <div class="field">
                  <div class="label">Your reply — send this back</div>
                  <textarea class="input input-mono" id="ctrl-answer-box" rows="3" readonly></textarea>
                  <button class="btn btn-ghost btn-sm" id="btn-ctrl-copy-answer">${icon("copy", "icon icon-sm")} Copy reply</button>
                </div>
                <span class="flow-status is-waiting"><span class="dot"></span>Waiting for them to accept…</span>
              </div>

              <div class="recent" id="recent-wrap">
                <div class="label">Recent</div>
                <div class="recent-list" id="recent-list"></div>
              </div>
            </section>

          </div>
        </div>
      </div>
    </div>`;
}
