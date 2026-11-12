export type RegisteredInterestStatus =
  { kind: "active" } | { kind: "deleted"; date: string; datetime: string };
