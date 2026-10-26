import { feedbackLimits } from "../feedback";

type FeedbackValues = { email: string; message: string };
export type FeedbackResult =
  | { kind: "success" }
  | {
      kind: "invalid";
      values: FeedbackValues;
      errors: { email: string; message: string };
    }
  | { kind: "unavailable" | "failed"; values: FeedbackValues };

type FeedbackConfig = {
  RESEND_API_KEY?: string;
  FEEDBACK_TO_EMAIL?: string;
};

class FeedbackInput {
  private constructor(
    readonly email: string,
    readonly message: string,
  ) {}

  static parse(
    data: FormData,
  ): FeedbackInput | Extract<FeedbackResult, { kind: "invalid" }> {
    const rawEmail = data.get("email");
    const rawMessage = data.get("message");
    const email = typeof rawEmail === "string" ? rawEmail.trim() : "";
    const message = typeof rawMessage === "string" ? rawMessage.trim() : "";
    const errors = {
      email:
        email.length <= feedbackLimits.email &&
        /^[^\s@<>]+@[^\s@<>]+\.[^\s@<>]+$/u.test(email)
          ? ""
          : "Enter a valid email address.",
      message:
        message.length > 0 && message.length <= feedbackLimits.message
          ? ""
          : `Enter a message of up to ${feedbackLimits.message} characters.`,
    };
    if (errors.email || errors.message) {
      return {
        kind: "invalid",
        values: {
          email: email.slice(0, feedbackLimits.email),
          message: message.slice(0, feedbackLimits.message),
        },
        errors,
      };
    }
    return new FeedbackInput(email, message);
  }
}

export async function sendFeedback(
  data: FormData,
  config: FeedbackConfig,
  fetcher: typeof fetch = fetch,
): Promise<FeedbackResult> {
  const input = FeedbackInput.parse(data);
  if (!(input instanceof FeedbackInput)) return input;
  const values = { email: input.email, message: input.message };
  const apiKey = config.RESEND_API_KEY?.trim();
  const recipient = config.FEEDBACK_TO_EMAIL?.trim();
  if (!apiKey || !recipient) {
    return { kind: "unavailable", values };
  }
  try {
    const response = await fetcher("https://api.resend.com/emails", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${apiKey}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        from: "onboarding@resend.dev",
        to: [recipient],
        reply_to: input.email,
        subject: "Exposed product feedback",
        text: input.message,
      }),
      signal: AbortSignal.timeout(10000),
    });
    return response.ok ? { kind: "success" } : { kind: "failed", values };
  } catch {
    return { kind: "failed", values };
  }
}
