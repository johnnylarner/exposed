import { fail } from "@sveltejs/kit";
import { sendFeedback } from "../../lib/server/feedback";
import type { Actions } from "./$types";

export const actions = {
  default: async ({ request, fetch }) => {
    const result = await sendFeedback(
      await request.formData(),
      process.env,
      fetch,
    );
    switch (result.kind) {
      case "success":
        return result;
      case "invalid":
        return fail(400, result);
      case "unavailable":
        return fail(503, result);
      case "failed":
        return fail(502, result);
    }
  },
} satisfies Actions;
