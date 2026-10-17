import { useTranslation } from "react-i18next";
import { Callout } from "@/components/ui/Callout";
import { AccessHelp } from "@/features/access/AccessHelp";
import type { UnreadableDir } from "@/lib/types";

/**
 * Places a rule points at that Sapu may not read (usually missing Full Disk
 * Access). Shown wherever results are, so "nothing found" or "done" is never
 * mistaken for "clean".
 */
export function UnreadablePlaces({ places }: { places: UnreadableDir[] }) {
  const { t } = useTranslation();
  if (places.length === 0) return null;
  return (
    <Callout tone="review" title={t("clean.unreadable_title", { count: places.length })}>
      <ul className="grid gap-0.5">
        {places.map((u) => (
          <li key={u.path} className="path truncate" title={u.reason}>
            {u.path}
          </li>
        ))}
      </ul>
      <AccessHelp />
    </Callout>
  );
}
