const { test, expect } = require("@playwright/test");

const BASE_URL = "http://localhost:3333";
const TIMEOUT = 120_000;

test.describe("rag3weaver WASM browser tests", () => {
  test("create + drain + count via Weaver embind class", async ({ page }) => {
    test.setTimeout(TIMEOUT);

    page.on("console", (msg) => console.log(`[browser] ${msg.text()}`));
    page.on("pageerror", (err) => console.error(`[browser error] ${err.message}`));

    await page.goto(`${BASE_URL}/weaver.html`);

    // Wait for all tests to complete
    await page.waitForFunction(
      () => window.testResults && (window.testResults.done === true || window.testResults.error),
      { timeout: TIMEOUT }
    );

    const results = await page.evaluate(() => window.testResults);
    console.log("Results:", JSON.stringify(results, null, 2));

    // Verify no error
    expect(results.error).toBeUndefined();

    // Test 1: version
    expect(results.version).toBe("0.1.0");

    // Test 2: constructor succeeded
    expect(results.constructorOk).toBe(true);

    // Test 3: 3 entities created (UUIDs are empty at create time — resolved after drain)
    expect(results.createCount).toBe(3);
    expect(results.createsOk).toBe(true);

    // Test 4: drain processed all 3
    expect(results.drain.processed).toBe(3);
    expect(results.drain.failed).toBe(0);

    // Test 5: count matches
    expect(results.count).toBe(3);

    // Test 6: async drain processed 2
    expect(results.asyncDrain.processed).toBe(2);
    expect(results.asyncDrain.failed).toBe(0);

    // Test 7: count after async drain = 5 total
    expect(results.countAfterAsync).toBe(5);

    // All done
    expect(results.done).toBe(true);
  });
});
