import type { Category } from "@/lib/types";

/** Sapu's own category glyphs: 24-grid, 1.75 stroke, rounded. Colour comes from the category token. */
const PATHS: Record<Category, React.ReactNode> = {
  video: (
    <>
      <rect x="3" y="5" width="18" height="14" rx="3" />
      <path d="M7 5v14M17 5v14M3 10h4M17 10h4M3 14h4M17 14h4" />
      <path d="M11 9.5v5l4-2.5z" fill="currentColor" stroke="none" />
    </>
  ),
  photo: (
    <>
      <rect x="3" y="4" width="18" height="16" rx="3" />
      <circle cx="9" cy="10" r="2" />
      <path d="M3 17l5-4 3 2.5 4-4.5 6 6" />
    </>
  ),
  document: (
    <>
      <path d="M7 3h7l5 5v11a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z" />
      <path d="M14 3v5h5M9 13h6M9 17h6" />
    </>
  ),
  application: (
    <>
      <rect x="3" y="3" width="7" height="7" rx="2" />
      <rect x="14" y="3" width="7" height="7" rx="2" />
      <rect x="3" y="14" width="7" height="7" rx="2" />
      <path d="M17.5 14v7M14 17.5h7" />
    </>
  ),
  system: (
    <>
      <rect x="7" y="7" width="10" height="10" rx="2" />
      <path d="M10 3v4M14 3v4M10 17v4M14 17v4M3 10h4M3 14h4M17 10h4M17 14h4" />
    </>
  ),
  temporary: (
    <>
      <circle cx="12" cy="12" r="8.5" />
      <path d="M12 7.5V12l3 2" />
      <path d="M3.5 12a8.5 8.5 0 0 0 2.6 6.1" strokeDasharray="2 3" />
    </>
  ),
  development: (
    <>
      <path d="M8 7l-5 5 5 5M16 7l5 5-5 5M13.5 5l-3 14" />
    </>
  ),
  other: (
    <>
      <circle cx="6" cy="12" r="1.75" fill="currentColor" stroke="none" />
      <circle cx="12" cy="12" r="1.75" fill="currentColor" stroke="none" />
      <circle cx="18" cy="12" r="1.75" fill="currentColor" stroke="none" />
      <rect x="3" y="5" width="18" height="14" rx="3" />
    </>
  ),
};

export function CategoryIcon({ category, size = 20, className }: { category: Category; size?: number; className?: string }) {
  return (
    <svg aria-hidden width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.75} strokeLinecap="round" strokeLinejoin="round" className={className}>
      {PATHS[category]}
    </svg>
  );
}
