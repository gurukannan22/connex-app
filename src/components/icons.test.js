// ============================================================
// components/icons.test.js — Tests for icon catalog & SVG generator
// ============================================================

import { test, describe } from "node:test";
import assert from "node:assert/strict";
import { icon, ICONS } from "./icons.js";

describe("icons component", () => {
  test("icon catalog contains required visual icons", () => {
    const required = ["monitor", "pointer", "copy", "refresh", "check", "share", "brand"];
    for (const key of required) {
      assert.ok(ICONS[key], `Icon catalog must contain '${key}'`);
      assert.ok(ICONS[key].includes("<path") || ICONS[key].includes("<rect"));
    }
  });

  test("icon function generates valid SVG string with default class", () => {
    const res = icon("monitor");
    assert.ok(res.startsWith('<svg class="icon" viewBox="0 0 24 24">'));
    assert.ok(res.endsWith("</svg>"));
    assert.ok(res.includes(ICONS.monitor));
  });

  test("icon function respects custom class name", () => {
    const res = icon("copy", "icon icon-sm custom-test");
    assert.ok(res.includes('class="icon icon-sm custom-test"'));
  });

  test("icon function handles unrecognized icon name safely", () => {
    const res = icon("non_existent_icon_name");
    assert.equal(res, '<svg class="icon" viewBox="0 0 24 24"></svg>');
  });
});
