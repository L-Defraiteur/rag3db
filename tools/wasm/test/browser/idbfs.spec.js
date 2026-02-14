const { test, expect } = require("@playwright/test");

const BASE_URL = "http://localhost:3333";
const TIMEOUT = 120_000;

test.describe("rag3db WASM IDBFS browser tests", () => {
  test("Phase 1: create + index + query + persist to IDBFS", async ({ page }) => {
    test.setTimeout(TIMEOUT);

    page.on("console", (msg) => console.log(`[browser] ${msg.text()}`));
    page.on("pageerror", (err) => console.error(`[browser error] ${err.message}`));

    await page.goto(`${BASE_URL}/?phase=1`);

    // Wait for phase 1 to complete
    await page.waitForFunction(
      () => window.testResults && window.testResults.phase1 === "done",
      { timeout: TIMEOUT }
    );

    const results = await page.evaluate(() => window.testResults);
    console.log("Phase 1 results:", JSON.stringify(results, null, 2));

    // Verify all tests passed
    expect(results.error).toBeUndefined();
    expect(results.contains).toBeGreaterThanOrEqual(1);
    expect(results.fuzzy).toBeGreaterThanOrEqual(1);
    expect(results.phrase).toBe(1);
    expect(results.vector).toBe(3);
    expect(results.phase1).toBe("done");
  });

  test("Phase 2: reload from IDBFS + verify persistence", async ({ page, context }) => {
    test.setTimeout(TIMEOUT);

    // First: run phase 1 to populate IDBFS
    page.on("console", (msg) => console.log(`[browser p1] ${msg.text()}`));
    await page.goto(`${BASE_URL}/?phase=1`);
    await page.waitForFunction(
      () => window.testResults && window.testResults.phase1 === "done",
      { timeout: TIMEOUT }
    );
    const p1 = await page.evaluate(() => window.testResults);
    expect(p1.phase1).toBe("done");
    console.log("Phase 1 done, reloading for phase 2...");

    // Phase 2: navigate to phase 2 (same context = same IndexedDB)
    page.removeAllListeners("console");
    page.on("console", (msg) => console.log(`[browser p2] ${msg.text()}`));

    await page.goto(`${BASE_URL}/?phase=2`);
    await page.waitForFunction(
      () => window.testResults && window.testResults.phase2 === "done",
      { timeout: TIMEOUT }
    );

    const results = await page.evaluate(() => window.testResults);
    console.log("Phase 2 results:", JSON.stringify(results, null, 2));

    // Verify persistence
    expect(results.error).toBeUndefined();
    expect(results.containsAfter).toBe(p1.contains);
    expect(results.fuzzyAfter).toBe(p1.fuzzy);
    expect(results.vectorAfter).toBe(p1.vector);
    expect(results.allDocsAfter).toBe(4);
    expect(results.phase2).toBe("done");

    // Verify same vector search results
    expect(results.vectorIdsAfter).toEqual(p1.vectorIds);
  });
});
