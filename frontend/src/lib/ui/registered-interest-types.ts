export type RegisteredInterestStatus =
  { kind: "active" } | { kind: "deleted"; date: string; datetime: string };

export type RegisteredInterestPresentation =
  { kind: "full" } | { kind: "compact"; summary: string };
