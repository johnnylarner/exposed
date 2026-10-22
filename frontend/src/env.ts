import { defineEnvVars } from "@sveltejs/kit/env";

export const variables = defineEnvVars({
  PUBLIC_APP_VERSION: {
    public: true,
    static: true,
    schema: (value) => value || "development",
  },
  PUBLIC_POSTHOG_PROJECT_TOKEN: {
    public: true,
    schema: (value) => value,
  },
  PUBLIC_POSTHOG_HOST: {
    public: true,
    schema: (value) => value,
  },
});
