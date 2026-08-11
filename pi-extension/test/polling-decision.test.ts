/**
 * polling.ts nextPollDelay 决策函数测试
 *
 * 导出：
 * ─────────────────────────────────────────────────────────
 * type TaskStatusQuery =
 *   | { kind: "ok" }
 *   | { kind: "rate_limited" }
 *   | { kind: "server_error" }
 *   | { kind: "network_error" }
 *   | { kind: "permanent" }
 *
 * type PollDecision = { delayMs: number; stop: boolean; consecutiveFailures: number }
 *
 * nextPollDelay(
 *   query: TaskStatusQuery,
 *   taskRuntimeMs: number,
 *   consecutiveFailures: number,
 *   retryAfterSeconds: number | null
 * ): PollDecision
 *
 * 常量：
 *   MIN_RETRY_MS: number  (预期 1000)
 *   MAX_RETRY_MS: number  (预期 300000)
 * ─────────────────────────────────────────────────────────
 */

import { describe, it } from "node:test";
import assert from "node:assert/strict";

import {
  nextPollDelay,
  MIN_RETRY_MS,
  MAX_RETRY_MS,
} from "../polling.ts";

// ─── 常量导出 ──────────────────────────────────────────────────────

describe("polling constants", () => {
  it("should export MIN_RETRY_MS as 1000", () => {
    assert.strictEqual(MIN_RETRY_MS, 1000);
  });

  it("should export MAX_RETRY_MS as 300000", () => {
    assert.strictEqual(MAX_RETRY_MS, 300_000);
  });
});

// ─── kind: "ok" ────────────────────────────────────────────────────

describe("nextPollDelay — kind: ok", () => {
  it("should reset consecutiveFailures to 0", () => {
    const result = nextPollDelay(
      { kind: "ok" },
      0,
      5, // had 5 consecutive failures
      null,
    );

    assert.strictEqual(result.consecutiveFailures, 0);
  });

  it("should use nextPollIntervalMs based on taskRuntimeMs (new task → 10000)", () => {
    const result = nextPollDelay(
      { kind: "ok" },
      60_000, // < 2 minutes
      0,
      null,
    );

    assert.strictEqual(result.delayMs, 10_000);
  });

  it("should use nextPollIntervalMs based on taskRuntimeMs (2~10 min → 20000)", () => {
    const result = nextPollDelay(
      { kind: "ok" },
      300_000, // 5 minutes
      0,
      null,
    );

    assert.strictEqual(result.delayMs, 20_000);
  });

  it("should use nextPollIntervalMs based on taskRuntimeMs (≥10 min → 30000)", () => {
    const result = nextPollDelay(
      { kind: "ok" },
      600_000, // 10 minutes
      0,
      null,
    );

    assert.strictEqual(result.delayMs, 30_000);
  });

  it("should not stop polling", () => {
    const result = nextPollDelay(
      { kind: "ok" },
      0,
      0,
      null,
    );

    assert.strictEqual(result.stop, false);
  });
});

// ─── kind: "rate_limited" ──────────────────────────────────────────

describe("nextPollDelay — kind: rate_limited", () => {
  describe("with Retry-After ≥ 1 (honored)", () => {
    it("should set delayMs to retryAfterSeconds * 1000", () => {
      const result = nextPollDelay(
        { kind: "rate_limited" },
        0,
        0,
        60, // 60 seconds
      );

      assert.strictEqual(result.delayMs, 60_000);
    });

    it("should not increment consecutiveFailures (controlled wait is not a failure)", () => {
      const result = nextPollDelay(
        { kind: "rate_limited" },
        0,
        3, // had 3 failures before
        120,
      );

      assert.strictEqual(result.consecutiveFailures, 3);
    });

    it("should not stop polling", () => {
      const result = nextPollDelay(
        { kind: "rate_limited" },
        0,
        0,
        30,
      );

      assert.strictEqual(result.stop, false);
    });

    it("should clamp small Retry-After to MIN_RETRY_MS (1 → 1000ms)", () => {
      const result = nextPollDelay(
        { kind: "rate_limited" },
        0,
        0,
        1, // 1 second → would be 1000ms, at boundary
      );

      assert.ok(result.delayMs >= MIN_RETRY_MS, `expected >= ${MIN_RETRY_MS}, got ${result.delayMs}`);
    });

    it("should clamp Retry-After of 0 to MIN_RETRY_MS (tight-loop prevention)", () => {
      // retryAfterSeconds=0 treated as "no usable value" → backoff fallback
      const result = nextPollDelay(
        { kind: "rate_limited" },
        0,
        0,
        0,
      );

      assert.ok(result.delayMs >= MIN_RETRY_MS, `expected >= ${MIN_RETRY_MS}, got ${result.delayMs}`);
    });

    it("should clamp large Retry-After to MAX_RETRY_MS (86400 → 300000ms)", () => {
      const result = nextPollDelay(
        { kind: "rate_limited" },
        0,
        0,
        86400, // 1 day
      );

      assert.strictEqual(result.delayMs, MAX_RETRY_MS);
    });
  });

  describe("without Retry-After (null or 0 → backoff fallback)", () => {
    it("should use nextBackoffMs(consecutiveFailures+1) when retryAfterSeconds is null", () => {
      const result = nextPollDelay(
        { kind: "rate_limited" },
        0,
        0, // current failures = 0 → next = 1 → nextBackoffMs(1) = 60000
        null,
      );

      assert.strictEqual(result.delayMs, 60_000);
    });

    it("should increment consecutiveFailures when retryAfterSeconds is null", () => {
      const result = nextPollDelay(
        { kind: "rate_limited" },
        0,
        2, // current failures = 2 → next = 3 → nextBackoffMs(3) = 240000
        null,
      );

      assert.strictEqual(result.consecutiveFailures, 3);
      assert.strictEqual(result.delayMs, 240_000);
    });

    it("should use backoff when retryAfterSeconds is 0 (no usable value)", () => {
      const result = nextPollDelay(
        { kind: "rate_limited" },
        0,
        0, // → next = 1 → nextBackoffMs(1) = 60000
        0,
      );

      assert.strictEqual(result.delayMs, 60_000);
      assert.strictEqual(result.consecutiveFailures, 1);
    });

    it("should not stop polling", () => {
      const result = nextPollDelay(
        { kind: "rate_limited" },
        0,
        0,
        null,
      );

      assert.strictEqual(result.stop, false);
    });
  });
});

// ─── kind: "server_error" ──────────────────────────────────────────

describe("nextPollDelay — kind: server_error", () => {
  it("should increment consecutiveFailures", () => {
    const result = nextPollDelay(
      { kind: "server_error" },
      0,
      1, // current = 1 → next = 2
      null,
    );

    assert.strictEqual(result.consecutiveFailures, 2);
  });

  it("should use nextBackoffMs(consecutiveFailures+1)", () => {
    const result = nextPollDelay(
      { kind: "server_error" },
      0,
      1, // → next = 2 → nextBackoffMs(2) = 120000
      null,
    );

    assert.strictEqual(result.delayMs, 120_000);
  });

  it("should not stop polling", () => {
    const result = nextPollDelay(
      { kind: "server_error" },
      0,
      0,
      null,
    );

    assert.strictEqual(result.stop, false);
  });
});

// ─── kind: "network_error" ─────────────────────────────────────────

describe("nextPollDelay — kind: network_error", () => {
  it("should increment consecutiveFailures", () => {
    const result = nextPollDelay(
      { kind: "network_error" },
      0,
      0, // → next = 1
      null,
    );

    assert.strictEqual(result.consecutiveFailures, 1);
  });

  it("should use nextBackoffMs(consecutiveFailures+1)", () => {
    const result = nextPollDelay(
      { kind: "network_error" },
      0,
      0, // → next = 1 → nextBackoffMs(1) = 60000
      null,
    );

    assert.strictEqual(result.delayMs, 60_000);
  });

  it("should not stop polling", () => {
    const result = nextPollDelay(
      { kind: "network_error" },
      0,
      0,
      null,
    );

    assert.strictEqual(result.stop, false);
  });
});

// ─── kind: "permanent" ─────────────────────────────────────────────

describe("nextPollDelay — kind: permanent", () => {
  it("should stop polling", () => {
    const result = nextPollDelay(
      { kind: "permanent" },
      0,
      0,
      null,
    );

    assert.strictEqual(result.stop, true);
  });

  it("should return delayMs of 0 (no need to schedule next poll)", () => {
    const result = nextPollDelay(
      { kind: "permanent" },
      0,
      0,
      null,
    );

    assert.strictEqual(result.delayMs, 0);
  });

  it("should not change consecutiveFailures", () => {
    const result = nextPollDelay(
      { kind: "permanent" },
      0,
      5,
      null,
    );

    assert.strictEqual(result.consecutiveFailures, 5);
  });
});

// ─── 连续失败序列推演 ───────────────────────────────────────────────

describe("nextPollDelay — consecutive failure sequence", () => {
  it("should produce delays of 60s, 120s, 240s for first 3 network_errors", () => {
    // 1st network_error: consecutiveFailures 0→1
    const r1 = nextPollDelay({ kind: "network_error" }, 0, 0, null);
    assert.strictEqual(r1.consecutiveFailures, 1);
    assert.strictEqual(r1.delayMs, 60_000);

    // 2nd network_error: consecutiveFailures 1→2
    const r2 = nextPollDelay({ kind: "network_error" }, 0, r1.consecutiveFailures, null);
    assert.strictEqual(r2.consecutiveFailures, 2);
    assert.strictEqual(r2.delayMs, 120_000);

    // 3rd network_error: consecutiveFailures 2→3
    const r3 = nextPollDelay({ kind: "network_error" }, 0, r2.consecutiveFailures, null);
    assert.strictEqual(r3.consecutiveFailures, 3);
    assert.strictEqual(r3.delayMs, 240_000);
  });

  it("should cap delay at 300s from 4th failure onward", () => {
    // 3rd failure: delayMs = 240000
    let failures = 2;
    const r3 = nextPollDelay({ kind: "network_error" }, 0, failures, null);
    failures = r3.consecutiveFailures;
    assert.strictEqual(failures, 3);
    assert.strictEqual(r3.delayMs, 240_000);

    // 4th failure: delayMs = 300000 (cap)
    const r4 = nextPollDelay({ kind: "network_error" }, 0, failures, null);
    failures = r4.consecutiveFailures;
    assert.strictEqual(failures, 4);
    assert.strictEqual(r4.delayMs, 300_000);

    // 5th failure: stays at 300000
    const r5 = nextPollDelay({ kind: "network_error" }, 0, failures, null);
    assert.strictEqual(r5.delayMs, 300_000);

    // 10th failure: still capped
    let current = r5.consecutiveFailures;
    for (let i = 0; i < 5; i++) {
      const r = nextPollDelay({ kind: "network_error" }, 0, current, null);
      current = r.consecutiveFailures;
      assert.strictEqual(r.delayMs, 300_000);
    }
  });

  it("should reset consecutiveFailures to 0 after ok following failures", () => {
    // Simulate 3 network_errors
    let failures = 0;
    for (let i = 0; i < 3; i++) {
      const r = nextPollDelay({ kind: "network_error" }, 0, failures, null);
      failures = r.consecutiveFailures;
    }
    assert.strictEqual(failures, 3);

    // Then ok
    const okResult = nextPollDelay({ kind: "ok" }, 0, failures, null);
    assert.strictEqual(okResult.consecutiveFailures, 0);
    assert.strictEqual(okResult.stop, false);
  });
});

// ─── 429 与网络错误混合 ────────────────────────────────────────────

describe("nextPollDelay — mixed 429 and network_error semantics", () => {
  it("should not increment failures on rate_limited with Retry-After, so subsequent network_error starts from base", () => {
    // Start: 0 failures
    // 1st: rate_limited with Retry-After → NOT a failure, stays at 0
    const r1 = nextPollDelay({ kind: "rate_limited" }, 0, 0, 60);
    assert.strictEqual(r1.consecutiveFailures, 0, "rate_limited with Retry-After should not increment");

    // 2nd: network_error → failures 0→1 → nextBackoffMs(1) = 60000
    const r2 = nextPollDelay({ kind: "network_error" }, 0, r1.consecutiveFailures, null);
    assert.strictEqual(r2.consecutiveFailures, 1);
    assert.strictEqual(r2.delayMs, 60_000, "network_error after non-incrementing 429 should start from 60s");
  });

  it("should increment failures on rate_limited without Retry-After, then network_error continues from there", () => {
    // Start: 0 failures
    // 1st: rate_limited without Retry-After → failures 0→1 → backoff = 60000
    const r1 = nextPollDelay({ kind: "rate_limited" }, 0, 0, null);
    assert.strictEqual(r1.consecutiveFailures, 1);
    assert.strictEqual(r1.delayMs, 60_000);

    // 2nd: network_error → failures 1→2 → backoff = 120000
    const r2 = nextPollDelay({ kind: "network_error" }, 0, r1.consecutiveFailures, null);
    assert.strictEqual(r2.consecutiveFailures, 2);
    assert.strictEqual(r2.delayMs, 120_000);
  });
});
