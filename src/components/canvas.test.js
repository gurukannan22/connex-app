// ============================================================
// components/canvas.test.js — Tests for canvas coordinate normalization
// ============================================================

import { test, describe } from "node:test";
import assert from "node:assert/strict";
import { normalizeCoordinates } from "./canvas.js";

describe("canvas component", () => {
  test("normalizeCoordinates calculates exact relative coordinates inside bounding box", () => {
    const rect = { left: 100, top: 50, width: 800, height: 600 };
    // Center point
    const center = normalizeCoordinates(500, 350, rect);
    assert.ok(center);
    assert.equal(center.x, 0.5);
    assert.equal(center.y, 0.5);

    // Top-left corner
    const topLeft = normalizeCoordinates(100, 50, rect);
    assert.ok(topLeft);
    assert.equal(topLeft.x, 0.0);
    assert.equal(topLeft.y, 0.0);

    // Bottom-right corner
    const bottomRight = normalizeCoordinates(900, 650, rect);
    assert.ok(bottomRight);
    assert.equal(bottomRight.x, 1.0);
    assert.equal(bottomRight.y, 1.0);
  });

  test("normalizeCoordinates clamps out-of-bounds coordinates to [0.0, 1.0]", () => {
    const rect = { left: 100, top: 100, width: 200, height: 200 };

    const tooFarLeft = normalizeCoordinates(50, 150, rect);
    assert.equal(tooFarLeft.x, 0.0);

    const tooFarRight = normalizeCoordinates(350, 150, rect);
    assert.equal(tooFarRight.x, 1.0);

    const tooFarAbove = normalizeCoordinates(150, 50, rect);
    assert.equal(tooFarAbove.y, 0.0);

    const tooFarBelow = normalizeCoordinates(150, 350, rect);
    assert.equal(tooFarBelow.y, 1.0);
  });

  test("normalizeCoordinates returns null when bounding box has zero dimensions", () => {
    assert.equal(normalizeCoordinates(10, 10, { left: 0, top: 0, width: 0, height: 100 }), null);
    assert.equal(normalizeCoordinates(10, 10, { left: 0, top: 0, width: 100, height: 0 }), null);
    assert.equal(normalizeCoordinates(10, 10, null), null);
  });
});
