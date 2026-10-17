const GB = 1024 ** 3;

/** Lifetime freed-space milestones in bytes; the badge level is the count reached. */
export const MILESTONES = [10 * GB, 50 * GB, 100 * GB, 250 * GB, 500 * GB, 1024 * GB];

export function milestoneIndex(freed: number): number {
  return MILESTONES.filter((m) => freed >= m).length;
}

/** Lifetime freed bytes at which Sapu asks for a rating, once. */
export const RATE_THRESHOLD = 100 * GB;
