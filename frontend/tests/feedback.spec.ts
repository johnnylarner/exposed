import { expect, test } from "@playwright/test";
import { sendFeedback } from "../src/lib/server/feedback";
const config = {
  RESEND_API_KEY: "test-key",
  FEEDBACK_TO_EMAIL: "feedback@example.com",
};
function submission(
  email = "visitor@example.com",
  message = "Please add more filters.",
) {
  const data = new FormData();
  data.set("email", email);
  data.set("message", message);
  return data;
}
test("invalid feedback never reaches the provider", async () => {
  for (const data of [
    submission("invalid"),
    submission("visitor@example.com", "  "),
    submission("visitor@example.com", "x".repeat(10001)),
  ]) {
    expect(
      (
        await sendFeedback(data, config, async () => {
          throw new Error("Provider must not be called");
        })
      ).kind,
    ).toBe("invalid");
  }
});
test("missing or blank feedback configuration retains input without contacting the provider", async () => {
  let providerCalls = 0;
  const provider: typeof fetch = async () => {
    providerCalls += 1;
    return Response.json({ id: "unexpected-email" });
  };
  for (const unavailableConfig of [
    {},
    { FEEDBACK_TO_EMAIL: config.FEEDBACK_TO_EMAIL },
    { ...config, RESEND_API_KEY: "  " },
    { RESEND_API_KEY: config.RESEND_API_KEY },
    { ...config, FEEDBACK_TO_EMAIL: "" },
    { ...config, FEEDBACK_TO_EMAIL: "  " },
  ])
    expect(await sendFeedback(submission(), unavailableConfig, provider)).toEqual({
      kind: "unavailable",
      values: {
        email: "visitor@example.com",
        message: "Please add more filters.",
      },
    });
  expect(providerCalls).toBe(0);
});
test("accepted feedback uses the configured recipient, onboarding sender and visitor reply-to", async () => {
  let request: Request | undefined;
  expect(
    await sendFeedback(submission(), config, async (url, options) => {
      request = new Request(url, options);
      return Response.json({ id: "accepted-email" });
    }),
  ).toEqual({ kind: "success" });
  expect(request?.url).toBe("https://api.resend.com/emails");
  expect(request?.headers.get("Authorization")).toBe("Bearer test-key");
  expect(await request?.json()).toEqual({
    from: "onboarding@resend.dev",
    to: [config.FEEDBACK_TO_EMAIL],
    reply_to: "visitor@example.com",
    subject: "Exposed product feedback",
    text: "Please add more filters.",
  });
});
test("provider rejection and network failure retain feedback", async () => {
  const providers: (typeof fetch)[] = [
    async () => new Response(null, { status: 429 }),
    async () => {
      throw new Error("Network unavailable");
    },
  ];
  for (const provider of providers)
    expect(await sendFeedback(submission(), config, provider)).toEqual({
      kind: "failed",
      values: {
        email: "visitor@example.com",
        message: "Please add more filters.",
      },
    });
});
