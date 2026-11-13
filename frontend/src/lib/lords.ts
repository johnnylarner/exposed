export interface RegisteredInterest {
  sourceId: number;
  text: string | null;
  createdWhen: string | null;
  lastAmendedWhen: string | null;
  deletedWhen: string | null;
  isCorrection: boolean;
  children: RegisteredInterest[];
}

export interface LordsCategory {
  sourceId: number;
  name: string | null;
  sortOrder: number;
  placement: "shareholdings" | "other";
  interests: RegisteredInterest[];
}

export interface LordsDetails {
  member: {
    id: string;
    name: string;
    partyName: string | null;
    membershipFrom: string | null;
    membershipStatus: string | null;
    membershipEndDate: string | null;
  };
  capturedAt: string;
  categories: LordsCategory[];
}

export type LordsState =
  | { kind: "lord"; details: LordsDetails }
  | { kind: "not-found" }
  | { kind: "error"; message: string };

export function groupLordsCategories(
  categories: LordsCategory[],
): LordsCategory[] {
  const groups = new Map<string, LordsCategory>();
  for (const category of categories) {
    const key = JSON.stringify([category.sourceId, category.name]);
    const existing = groups.get(key);
    if (existing) {
      existing.interests.push(...category.interests);
      existing.sortOrder = Math.min(existing.sortOrder, category.sortOrder);
    } else {
      groups.set(key, { ...category, interests: [...category.interests] });
    }
  }
  return [...groups.values()].sort(
    (left, right) => left.sortOrder - right.sortOrder,
  );
}

export function currentLordsCategories(
  categories: LordsCategory[],
): LordsCategory[] {
  function currentInterests(
    interests: RegisteredInterest[],
  ): RegisteredInterest[] {
    return interests.flatMap((interest) => {
      const children = currentInterests(interest.children);
      return interest.deletedWhen === null
        ? [{ ...interest, children }]
        : children;
    });
  }
  return categories
    .map((category) => ({
      ...category,
      interests: currentInterests(category.interests),
    }))
    .filter((category) => category.interests.length > 0);
}

export function sourceCalendarDate(value: string): string {
  const [year, month, day] = value.slice(0, 10).split("-").map(Number);
  const date = new Date(0);
  date.setUTCFullYear(year, month - 1, day);
  date.setUTCHours(0, 0, 0, 0);
  return new Intl.DateTimeFormat("en-GB", {
    day: "numeric",
    month: "short",
    year: "numeric",
    timeZone: "UTC",
  }).format(date);
}
