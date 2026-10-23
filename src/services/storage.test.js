// ============================================================
// services/storage.test.js — Tests for recent connections storage
// ============================================================

import { test, describe } from "node:test";
import assert from "node:assert/strict";
import { isValidDeviceId, getRecent, saveRecent, clearRecent, RECENT_KEY, MAX_RECENT_ITEMS } from "./storage.js";

// Mock localStorage for headless Node environment
function createMockStorage(initial = {}) {
  const store = new Map(Object.entries(initial));
  return {
    getItem: (k) => store.get(k) ?? null,
    setItem: (k, v) => store.set(k, String(v)),
    removeItem: (k) => store.delete(k),
    clear: () => store.clear(),
  };
}

describe("storage service", () => {
  test("isValidDeviceId validates 6-digit numeric IDs", () => {
    assert.equal(isValidDeviceId("592104"), true);
    assert.equal(isValidDeviceId("000000"), true);
    assert.equal(isValidDeviceId("12345"), false);
    assert.equal(isValidDeviceId("1234567"), false);
    assert.equal(isValidDeviceId("abc123"), false);
    assert.equal(isValidDeviceId(null), false);
    assert.equal(isValidDeviceId(undefined), false);
  });

  test("getRecent returns empty array on empty storage", () => {
    const storage = createMockStorage();
    assert.deepEqual(getRecent(storage), []);
  });

  test("getRecent recovers gracefully from corrupted JSON", () => {
    const storage = createMockStorage({ [RECENT_KEY]: "invalid-json{{{" });
    assert.deepEqual(getRecent(storage), []);
  });

  test("saveRecent prepends ID and removes duplicate occurrences", () => {
    const storage = createMockStorage();
    saveRecent("111111", storage);
    saveRecent("222222", storage);
    const list = saveRecent("111111", storage);

    assert.deepEqual(list, ["111111", "222222"]);
  });

  test("saveRecent limits list to MAX_RECENT_ITEMS", () => {
    const storage = createMockStorage();
    for (let i = 1; i <= 10; i++) {
      saveRecent(`10000${i % 10}`, storage);
    }
    const list = getRecent(storage);
    assert.equal(list.length, MAX_RECENT_ITEMS);
  });

  test("saveRecent rejects invalid IDs without modifying state", () => {
    const storage = createMockStorage();
    saveRecent("123456", storage);
    saveRecent("not-an-id", storage);
    assert.deepEqual(getRecent(storage), ["123456"]);
  });

  test("clearRecent clears stored connections", () => {
    const storage = createMockStorage();
    saveRecent("123456", storage);
    clearRecent(storage);
    assert.deepEqual(getRecent(storage), []);
  });
});
