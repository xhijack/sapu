import { useTranslation } from "react-i18next";
import { ExternalLink, FolderSearch } from "lucide-react";
import { Button } from "@/components/ui/button";
import { api } from "@/lib/api";
import { usePermissionOwner } from "./permissionOwner";

/**
 * How to give Full Disk Access, naming the app that really needs it: Sapu
 * itself, or (for a development build) the editor or terminal it runs from.
 */
export function AccessHelp() {
  const { t } = useTranslation();
  const owner = usePermissionOwner();
  const text = owner ? (owner.inherited ? t("access.inherited", { name: owner.name }) : t("access.own", { name: owner.name })) : t("access.unknown");
  return (
    <div className="grid gap-2">
      <p>{text}</p>
      <div className="flex flex-wrap gap-2">
        <Button variant="secondary" size="sm" onClick={() => void api.openDiskAccessSettings()}>
          <ExternalLink aria-hidden className="h-4 w-4" />
          {t("access.open_settings")}
        </Button>
        {owner && (
          <Button variant="ghost" size="sm" onClick={() => void api.revealAppBinary()}>
            <FolderSearch aria-hidden className="h-4 w-4" />
            {t("access.reveal", { name: owner.name })}
          </Button>
        )}
      </div>
    </div>
  );
}
