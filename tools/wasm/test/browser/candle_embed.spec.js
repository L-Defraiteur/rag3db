const { test, expect } = require("@playwright/test");

const BASE_URL = "http://localhost:3333";
const TIMEOUT = 180_000; // 3 min (model download ~23MB + WASM inference)

test.describe("Candle embedding in WASM", () => {
  test("all-MiniLM-L6-v2 produces valid 384d embeddings via candle in WASM", async ({ page }) => {
    test.setTimeout(TIMEOUT);

    page.on("console", (msg) => console.log(`[browser] ${msg.text()}`));
    page.on("pageerror", (err) => console.error(`[browser error] ${err.message}`));

    await page.goto(`${BASE_URL}/candle_embed.html`);

    // Wait for the test result (model download + inference can be slow)
    await page.waitForFunction(
      () => window.testResult,
      { timeout: TIMEOUT }
    );

    const result = await page.evaluate(() => window.testResult);
    console.log("Candle WASM result:", JSON.stringify(result, null, 2));

    // Core assertions
    expect(result.ok).toBe(true);
    expect(result.dim).toBe(384);
    expect(result.nonZero).toBeGreaterThan(300); // most dims are non-zero
    expect(result.norm).toBeGreaterThan(0.95);   // L2-normalized, norm ~1.0
    expect(result.norm).toBeLessThan(1.05);
    expect(result.sample).toHaveLength(3);       // first 3 values returned
  });
});
