const { defineConfig } = require("@playwright/test");

module.exports = defineConfig({
  testDir: "./test/browser",
  testMatch: "*.spec.js",
  timeout: 120_000,
  use: {
    headless: true,
    browserName: "chromium",
  },
  webServer: {
    command: "node test/browser/serve.js",
    port: 3333,
    reuseExistingServer: true,
    timeout: 10_000,
  },
});
