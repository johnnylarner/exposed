import { defineConfig } from "@playwright/test";
import config from "./playwright.config";

if (!process.env.EXPOSED_CONTRACT_OUTPUT_DIR)
  throw new Error(
    "EXPOSED_CONTRACT_OUTPUT_DIR must contain fresh Rust HTTP responses",
  );

export default defineConfig({
  ...config,
  testMatch: "declared-totals.contract.ts",
});
