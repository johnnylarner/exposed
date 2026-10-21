import { defineEnvVars } from "@sveltejs/kit/env";

export const variables = defineEnvVars({
  PUBLIC_APP_VERSION: {
    public: true,
    static: true,
    schema: (value) => value || "development",
  },
});
