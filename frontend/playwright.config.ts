import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  projects: [
    {
      name: "unit",
      testMatch:
        /(?:member-summary|funder-summary|loader-adapters|updates-sort|feedback)\.spec\.ts$/,
    },
    {
      name: "browser",
      testIgnore:
        /(?:member-summary|funder-summary|loader-adapters|updates-sort|feedback)\.spec\.ts$/,
    },
  ],
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  reporter: "list",
  use: {
    baseURL: "http://127.0.0.1:4173",
    ...devices["Desktop Chrome"],
    trace: "retain-on-failure",
  },
});
