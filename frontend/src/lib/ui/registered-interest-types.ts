export type RegisteredInterestStatus =
  { kind: "active" } | { kind: "deleted"; date: string; datetime: string };

export type RegisteredInterestVariant = "default" | "company";
