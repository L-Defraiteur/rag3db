const { test, expect } = require("@playwright/test");

const BASE_URL = "http://localhost:3333";

test.describe("setEmbedder — swap MockEmbedder for CandleEmbedder in WASM", () => {

  test("MiniLM — all-MiniLM-L6-v2 (~23MB)", async ({ page }) => {
    test.setTimeout(180_000);

    page.on("console", (msg) => console.log(`[browser] ${msg.text()}`));
    page.on("pageerror", (err) => console.error(`[browser error] ${err.message}`));

    await page.goto(`${BASE_URL}/set_embedder.html?model=minilm`);

    await page.waitForFunction(
      () => window.testResults && (window.testResults.done || window.testResults.error),
      { timeout: 180_000 }
    );

    const r = await page.evaluate(() => window.testResults);
    console.log("MiniLM results:", JSON.stringify(r, null, 2));

    expect(r.error).toBeUndefined();
    expect(r.constructorOk).toBe(true);
    expect(r.setEmbedder.ok).toBe(true);
    expect(r.drain.processed).toBe(3);
    expect(r.drain.failed).toBe(0);
    expect(r.search).toBeDefined();
    expect(r.search.results.length).toBeGreaterThan(0);
    expect(r.done).toBe(true);
  });

  test("Multilingual — paraphrase-multilingual-MiniLM-L12-v2 (~471MB)", async ({ page }) => {
    test.setTimeout(600_000);

    page.on("console", (msg) => console.log(`[browser] ${msg.text()}`));
    page.on("pageerror", (err) => console.error(`[browser error] ${err.message}`));

    await page.goto(`${BASE_URL}/set_embedder.html?model=multilingual`);

    await page.waitForFunction(
      () => window.testResults && (window.testResults.done || window.testResults.error),
      { timeout: 600_000 }
    );

    const r = await page.evaluate(() => window.testResults);
    console.log("Multilingual results:", JSON.stringify(r, null, 2));

    expect(r.error).toBeUndefined();
    expect(r.constructorOk).toBe(true);
    expect(r.setEmbedder.ok).toBe(true);
    expect(r.drain.processed).toBe(3);
    expect(r.drain.failed).toBe(0);
    expect(r.search).toBeDefined();
    expect(r.search.results.length).toBeGreaterThan(0);
    expect(r.done).toBe(true);
  });

});
