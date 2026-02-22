const { test, expect } = require("@playwright/test");

const BASE_URL = "http://localhost:3333";
const TIMEOUT = 120_000;

test.describe("rag3weaver WASM browser tests", () => {
  test("opaque handles + drain resolved + link + getUuid", async ({ page }) => {
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

    // Test 3: 3 entities created with int handles (0, 1, 2)
    expect(results.createCount).toBe(3);
    expect(results.createsOk).toBe(true);
    expect(results.handles).toEqual([0, 1, 2]);

    // Test 3b: getUuid before drain = pending
    expect(results.uuidBeforeDrain.error).toBe("pending");

    // Test 4: drain processed all 3, resolved array present
    expect(results.drain.processed).toBe(3);
    expect(results.drain.failed).toBe(0);
    expect(results.drain.resolved).toBeDefined();
    expect(results.drain.resolved.length).toBe(3);
    // Each resolved item has handle, entity, uuid
    for (const r of results.drain.resolved) {
      expect(r.handle).toBeGreaterThanOrEqual(0);
      expect(r.entity).toBe("Document");
      expect(r.uuid).toBeTruthy();
      expect(r.uuid.length).toBeGreaterThan(0);
    }

    // Test 4b: getUuid after drain = valid UUID
    expect(results.uuidAfterDrain.uuid).toBeTruthy();
    expect(results.uuidAfterDrain.uuid.length).toBeGreaterThan(0);

    // Test 5: count matches
    expect(results.count).toBe(3);

    // Test 6: link succeeded + async drain
    expect(results.linkResult).toBe(0);
    expect(results.asyncDrain.processed).toBeGreaterThanOrEqual(2);
    expect(results.asyncDrain.failed).toBe(0);
    // Async drain resolved should include all 5 handles (cumulative)
    expect(results.asyncDrain.resolved).toBeDefined();
    expect(results.asyncDrain.resolved.length).toBe(5);

    // Test 7: count after async drain = 5 total
    expect(results.countAfterAsync).toBe(5);

    // Test 8: async handles have UUIDs after drain
    expect(results.asyncUuids.h4.uuid).toBeTruthy();
    expect(results.asyncUuids.h5.uuid).toBeTruthy();

    // Test 9: search async — full response with meta
    expect(results.search).toBeDefined();
    expect(results.search.error).toBeUndefined();
    expect(Array.isArray(results.search.results)).toBe(true);
    expect(results.search.meta).toBeDefined();
    expect(results.search.meta.query).toBe("test query");
    expect(results.search.meta.kb).toBe("main");

    // All done
    expect(results.done).toBe(true);
  });
});
