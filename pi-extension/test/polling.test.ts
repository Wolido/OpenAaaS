/**
 * polling.ts 纯函数模块测试
 *
 * 模块路径: ../polling.ts（与 index.ts 同目录）
 *
 * 导出规格：
 * ─────────────────────────────────────────────────────────
 * 1. nextPollIntervalMs(taskAgeMs: number): number
 *    按任务年龄（毫秒，从服务端 created_at 算起）返回下次轮询间隔：
 *    - 0 ≤ age < 120_000（< 2 分钟）   → 10_000
 *    - 120_000 ≤ age < 600_000（2~10 分钟）→ 20_000
 *    - age ≥ 600_000（≥ 10 分钟）       → 30_000
 *
 * 2. nextBackoffMs(consecutiveFailures: number): number
 *    连续失败退避（网络错误 / 5xx，不含 429）：
 *    - 0 → 60_000（调用方保证 ≥1，但函数对 0 也返回合理值）
 *    - 1 → 60_000
 *    - 2 → 120_000
 *    - 3 → 240_000
 *    - 4+ → 300_000（封顶）
 *
 * 3. parseRetryAfterSeconds(headerValue: string | null): number | null
 *    解析 HTTP 429 Retry-After 头（仅支持秒数格式）：
 *    - 合法非负整数秒数字符串 → 对应数字
 *    - 含前导/尾随空格 → trim 后解析
 *    - 非数字 / 空字符串 / null / HTTP 日期格式 → null
 * ─────────────────────────────────────────────────────────
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  nextPollIntervalMs,
  nextBackoffMs,
  parseRetryAfterSeconds,
} from "../polling.ts";

// ─── nextPollIntervalMs ────────────────────────────────────────────

describe("nextPollIntervalMs", () => {
  describe("第一档：0 ≤ age < 120_000 → 10_000", () => {
    it("should return 10000 when age is 0 (brand new task)", () => {
      assert.strictEqual(nextPollIntervalMs(0), 10_000);
    });

    it("should return 10000 when age is 119_999 (just below 2-minute boundary)", () => {
      assert.strictEqual(nextPollIntervalMs(119_999), 10_000);
    });

    it("should return 10000 when age is 60_000 (mid-range of first tier)", () => {
      assert.strictEqual(nextPollIntervalMs(60_000), 10_000);
    });

    it("should return 10000 when age is 1 (near zero)", () => {
      assert.strictEqual(nextPollIntervalMs(1), 10_000);
    });
  });

  describe("第二档：120_000 ≤ age < 600_000 → 20_000", () => {
    it("should return 20000 when age is exactly 120_000 (boundary)", () => {
      assert.strictEqual(nextPollIntervalMs(120_000), 20_000);
    });

    it("should return 20000 when age is 599_999 (just below 10-minute boundary)", () => {
      assert.strictEqual(nextPollIntervalMs(599_999), 20_000);
    });

    it("should return 20000 when age is 300_000 (mid-range of second tier)", () => {
      assert.strictEqual(nextPollIntervalMs(300_000), 20_000);
    });
  });

  describe("第三档：age ≥ 600_000 → 30_000", () => {
    it("should return 30000 when age is exactly 600_000 (boundary)", () => {
      assert.strictEqual(nextPollIntervalMs(600_000), 30_000);
    });

    it("should return 30000 when age is 1_000_000 (well above 10 minutes)", () => {
      assert.strictEqual(nextPollIntervalMs(1_000_000), 30_000);
    });

    it("should return 30000 when age is 3_600_000 (1 hour)", () => {
      assert.strictEqual(nextPollIntervalMs(3_600_000), 30_000);
    });
  });
});

// ─── nextBackoffMs ─────────────────────────────────────────────────

describe("nextBackoffMs", () => {
  it("should return 60000 when consecutiveFailures is 0 (edge case, caller guarantees ≥1)", () => {
    assert.strictEqual(nextBackoffMs(0), 60_000);
  });

  it("should return 60000 when consecutiveFailures is 1", () => {
    assert.strictEqual(nextBackoffMs(1), 60_000);
  });

  it("should return 120000 when consecutiveFailures is 2", () => {
    assert.strictEqual(nextBackoffMs(2), 120_000);
  });

  it("should return 240000 when consecutiveFailures is 3", () => {
    assert.strictEqual(nextBackoffMs(3), 240_000);
  });

  it("should return 300000 when consecutiveFailures is 4 (cap reached)", () => {
    assert.strictEqual(nextBackoffMs(4), 300_000);
  });

  it("should return 300000 when consecutiveFailures is 5 (stays at cap)", () => {
    assert.strictEqual(nextBackoffMs(5), 300_000);
  });

  it("should return 300000 when consecutiveFailures is 100 (large value, stays at cap)", () => {
    assert.strictEqual(nextBackoffMs(100), 300_000);
  });

  it("should return 300000 when consecutiveFailures is 1000 (extreme value, stays at cap)", () => {
    assert.strictEqual(nextBackoffMs(1000), 300_000);
  });
});

// ─── parseRetryAfterSeconds ────────────────────────────────────────

describe("parseRetryAfterSeconds", () => {
  describe("valid numeric strings", () => {
    it('should return 60 when input is "60"', () => {
      assert.strictEqual(parseRetryAfterSeconds("60"), 60);
    });

    it('should return 120 when input is "120"', () => {
      assert.strictEqual(parseRetryAfterSeconds("120"), 120);
    });

    it('should return 0 when input is "0"', () => {
      assert.strictEqual(parseRetryAfterSeconds("0"), 0);
    });

    it('should return 1 when input is "1"', () => {
      assert.strictEqual(parseRetryAfterSeconds("1"), 1);
    });

    it('should return 3600 when input is "3600" (1 hour)', () => {
      assert.strictEqual(parseRetryAfterSeconds("3600"), 3600);
    });
  });

  describe("whitespace handling", () => {
    it('should return 60 when input is " 60 " (leading and trailing spaces)', () => {
      assert.strictEqual(parseRetryAfterSeconds(" 60 "), 60);
    });

    it('should return 60 when input is "  60" (leading spaces only)', () => {
      assert.strictEqual(parseRetryAfterSeconds("  60"), 60);
    });

    it('should return 60 when input is "60  " (trailing spaces only)', () => {
      assert.strictEqual(parseRetryAfterSeconds("60  "), 60);
    });

    it('should return 60 when input is "\\t60\\n" (tab and newline)', () => {
      assert.strictEqual(parseRetryAfterSeconds("\t60\n"), 60);
    });
  });

  describe("invalid inputs → null", () => {
    it("should return null when input is null", () => {
      assert.strictEqual(parseRetryAfterSeconds(null), null);
    });

    it('should return null when input is "" (empty string)', () => {
      assert.strictEqual(parseRetryAfterSeconds(""), null);
    });

    it('should return null when input is "abc" (non-numeric)', () => {
      assert.strictEqual(parseRetryAfterSeconds("abc"), null);
    });

    it('should return null when input is "  " (whitespace only)', () => {
      assert.strictEqual(parseRetryAfterSeconds("  "), null);
    });

    it('should return null when input is "12abc" (mixed content)', () => {
      assert.strictEqual(parseRetryAfterSeconds("12abc"), null);
    });
  });

  describe("HTTP date format → null (not supported)", () => {
    it('should return null for HTTP date "Wed, 21 Oct 2015 07:28:00 GMT"', () => {
      assert.strictEqual(
        parseRetryAfterSeconds("Wed, 21 Oct 2015 07:28:00 GMT"),
        null,
      );
    });

    it('should return null for ISO date "2024-01-01T00:00:00Z"', () => {
      assert.strictEqual(
        parseRetryAfterSeconds("2024-01-01T00:00:00Z"),
        null,
      );
    });
  });
});
