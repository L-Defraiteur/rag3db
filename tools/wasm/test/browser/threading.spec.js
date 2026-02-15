const { test, expect } = require("@playwright/test");

const BASE_URL = "http://localhost:3333";
const TIMEOUT = 120_000;

test.describe("WASM threading validation", () => {
  test("Test A: std::thread::spawn + atomics", async ({ page }) => {
    test.setTimeout(TIMEOUT);

    page.on("console", (msg) => console.log(`[browser] ${msg.text()}`));
    page.on("pageerror", (err) => console.error(`[browser error] ${err.message}`));

    await page.goto(`${BASE_URL}/threading.html`);

    await page.waitForFunction(
      () => window.testResults && (window.testResults.done === true || window.testResults.error),
      { timeout: TIMEOUT }
    );

    const results = await page.evaluate(() => window.testResults);
    console.log("Results:", JSON.stringify(results, null, 2));

    expect(results.error).toBeUndefined();

    // Test A: std::thread::spawn
    expect(results.threads).toBeDefined();
    expect(results.threads.ok).toBe(true);
    expect(results.threads.total).toBe(4000);
    expect(results.threads.expected).toBe(4000);
    expect(results.threads.threads).toBe(4);
    expect(results.threads.joinErrors).toBe(0);
  });

  test("Test B: futures::executor::ThreadPool", async ({ page }) => {
    test.setTimeout(TIMEOUT);

    page.on("console", (msg) => console.log(`[browser] ${msg.text()}`));
    page.on("pageerror", (err) => console.error(`[browser error] ${err.message}`));

    await page.goto(`${BASE_URL}/threading.html`);

    await page.waitForFunction(
      () => window.testResults && (window.testResults.done === true || window.testResults.error),
      { timeout: TIMEOUT }
    );

    const results = await page.evaluate(() => window.testResults);
    console.log("Results:", JSON.stringify(results, null, 2));

    expect(results.error).toBeUndefined();

    // Test B: futures::executor::ThreadPool
    expect(results.asyncPool).toBeDefined();
    expect(results.asyncPool.ok).toBe(true);
    expect(results.asyncPool.total).toBe(8);
    expect(results.asyncPool.expected).toBe(8);
    expect(results.asyncPool.poolSize).toBe(2);
  });

  test("Test C: rayon par_iter", async ({ page }) => {
    test.setTimeout(TIMEOUT);

    page.on("console", (msg) => console.log(`[browser] ${msg.text()}`));
    page.on("pageerror", (err) => console.error(`[browser error] ${err.message}`));

    await page.goto(`${BASE_URL}/threading.html`);

    await page.waitForFunction(
      () => window.testResults && (window.testResults.done === true || window.testResults.error),
      { timeout: TIMEOUT }
    );

    const results = await page.evaluate(() => window.testResults);
    console.log("Results:", JSON.stringify(results, null, 2));

    expect(results.error).toBeUndefined();

    // Test C: rayon par_iter
    expect(results.rayon).toBeDefined();
    expect(results.rayon.ok).toBe(true);
    expect(results.rayon.total).toBe(500500); // sum 1..=1000
    expect(results.rayon.expected).toBe(500500);
    expect(results.rayon.numChunks).toBe(4); // 1000/250
  });

  // Test D (tokio) retiré — épuise le pool de threads emscripten partagé.
  // tokio enable_all() crée des threads internes (timer, signal) qui
  // consomment les 16 workers déjà partagés avec Kuzu + tantivy.
});
