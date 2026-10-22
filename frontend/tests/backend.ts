import { test as base, expect } from "@playwright/test";
import { createServer, type ServerResponse } from "node:http";
import { spawn } from "node:child_process";
import { once } from "node:events";

type Reply = {
  status?: number;
  body?: string;
  json?: unknown;
  contentType?: string;
};
type BackendRoute = {
  request(): { url(): string };
  fulfill(reply: Reply): Promise<void>;
};
export class Backend {
  handlers: { pattern: string; handle: (route: BackendRoute) => unknown }[] =
    [];
  waiters: {
    predicate: (request: { url(): string }) => boolean;
    resolve: () => void;
  }[] = [];
  async route(pattern: string, handle: (route: BackendRoute) => unknown) {
    this.handlers.push({ pattern, handle });
  }
  async unroute(pattern: string) {
    this.handlers = this.handlers.filter(
      (handler) => handler.pattern !== pattern,
    );
  }
  waitForRequest(predicate: (request: { url(): string }) => boolean) {
    return new Promise<void>((resolve) =>
      this.waiters.push({ predicate, resolve }),
    );
  }
  async respond(url: string, response: ServerResponse) {
    const request = { url: () => url };
    for (const waiter of this.waiters)
      if (waiter.predicate(request)) waiter.resolve();
    const handler = this.handlers.findLast(({ pattern }) =>
      new RegExp(
        "^" +
          pattern
            .split("*")
            .map((part) => part.replace(/[.+?^${}()|[\]\\]/g, "\\$&"))
            .join(".*") +
          "$",
      ).test(url),
    );
    if (!handler) {
      response.writeHead(404).end();
      return;
    }
    await handler.handle({
      request: () => request,
      fulfill: async ({ status = 200, body, json, contentType }) => {
        response.writeHead(status, {
          "content-type":
            contentType ??
            (json === undefined ? "text/plain" : "application/json"),
        });
        response.end(json === undefined ? body : JSON.stringify(json));
      },
    });
  }
}
export const test = base.extend<
  { backend: Backend },
  {
    analytics: boolean;
    development: boolean;
    application: { url: string; backend: Backend };
  }
>({
  analytics: [false, { option: true, scope: "worker" }],
  development: [false, { option: true, scope: "worker" }],
  application: [
    async ({ analytics, development }, use, workerInfo) => {
      const backend = new Backend();
      const server = createServer((req, res) => {
        void backend.respond(`http://127.0.0.1/api${req.url}`, res);
      });
      server.listen(0, "127.0.0.1");
      await once(server, "listening");
      const address = server.address();
      if (!address || typeof address === "string")
        throw new Error("No backend port");
      const port = 4300 + workerInfo.workerIndex;
      const child = spawn(
        process.execPath,
        development
          ? [
              "node_modules/vite/bin/vite.js",
              "--host",
              "127.0.0.1",
              "--port",
              String(port),
              "--strictPort",
            ]
          : ["build"],
        {
          env: {
            ...process.env,
            PUBLIC_POSTHOG_PROJECT_TOKEN: analytics
              ? "phc_test_search_tracking"
              : "",
            PUBLIC_POSTHOG_HOST: analytics ? "https://posthog.test" : "",
            PORT: String(port),
            HOST: "127.0.0.1",
            EXPOSED_API_ORIGIN: `http://127.0.0.1:${address.port}`,
          },
          stdio: "pipe",
        },
      );
      const url = `http://127.0.0.1:${port}`;
      try {
        await expect
          .poll(
            async () => {
              try {
                return (await fetch(url)).status;
              } catch {
                return 0;
              }
            },
            { timeout: 15000 },
          )
          .toBe(200);
        await use({ url, backend });
      } finally {
        child.kill("SIGTERM");
        await once(child, "exit");
        server.closeAllConnections();
        await new Promise<void>((resolve) => server.close(() => resolve()));
      }
    },
    { scope: "worker" },
  ],
  page: async ({ page }, use) => {
    await page.route("https://posthog.test/**", (route) =>
      route.fulfill({ json: {} }),
    );
    await use(page);
  },
  baseURL: async ({ application }, use) => {
    await use(application.url);
  },
  backend: [
    async ({ application }, use) => {
      application.backend.handlers = [];
      application.backend.waiters = [];
      await use(application.backend);
    },
    { auto: true },
  ],
});
export { expect };
