import type { Category } from "@/lib/types";

/** Category hue as a CSS variable reference; always paired with a label in the UI. */
export const categoryColor = (c: Category) => `var(--cat-${c})`;
