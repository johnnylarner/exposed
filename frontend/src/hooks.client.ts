import { dev } from "$app/env";
import {
  PUBLIC_POSTHOG_HOST,
  PUBLIC_POSTHOG_PROJECT_TOKEN,
} from "$app/env/public";
import type { HandleClientError } from "@sveltejs/kit/hooks";
import posthog from "posthog-js";

export function init() {
  if (dev) return;

  const projectToken = PUBLIC_POSTHOG_PROJECT_TOKEN;
  const host = PUBLIC_POSTHOG_HOST;

  if (!projectToken || !host) {
    return;
  }

  posthog.init(projectToken, {
    api_host: host,
    logs: {
      serviceName: "exposed-frontend",
      environment: "production",
    },
    capture_exceptions: {
      capture_unhandled_errors: true,
      capture_unhandled_rejections: true,
      capture_console_errors: false,
    },
  });
}

export const handleError: HandleClientError = ({ error }) => {
  if (!dev && PUBLIC_POSTHOG_PROJECT_TOKEN && PUBLIC_POSTHOG_HOST) {
    posthog.captureException(error);
  }
};
