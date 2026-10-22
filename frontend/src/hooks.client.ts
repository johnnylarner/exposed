import { dev } from "$app/env";
import {
  PUBLIC_POSTHOG_HOST,
  PUBLIC_POSTHOG_PROJECT_TOKEN,
} from "$app/env/public";
import type { HandleClientError } from "@sveltejs/kit/hooks";
import posthog from "posthog-js";

export function init() {
  const projectToken = PUBLIC_POSTHOG_PROJECT_TOKEN;
  const host = PUBLIC_POSTHOG_HOST;

  if (!projectToken) {
    if (dev) {
      throw new Error(
        "PUBLIC_POSTHOG_PROJECT_TOKEN variable required by PostHog is missing or un-configured, this causes events to be silently missed. This error stops appearing once PUBLIC_POSTHOG_PROJECT_TOKEN is configured",
      );
    }
    return;
  }

  if (!host) {
    if (dev) {
      throw new Error(
        "PUBLIC_POSTHOG_HOST variable required by PostHog is missing or un-configured, this causes events to be silently missed. This error stops appearing once PUBLIC_POSTHOG_HOST is configured",
      );
    }
    return;
  }

  posthog.init(projectToken, {
    api_host: host,
    logs: {
      serviceName: "exposed-frontend",
      environment: dev ? "development" : "production",
    },
    capture_exceptions: {
      capture_unhandled_errors: true,
      capture_unhandled_rejections: true,
      capture_console_errors: false,
    },
  });
}

export const handleError: HandleClientError = ({ error }) => {
  if (PUBLIC_POSTHOG_PROJECT_TOKEN && PUBLIC_POSTHOG_HOST) {
    posthog.captureException(error);
  }
};
