import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { hierarchy, treemap, treemapSquarify, type HierarchyRectangularNode } from "d3-hierarchy";
import { ArrowUp } from "lucide-react";
import { Button } from "@/components/ui/button";
import { LoadingState } from "@/components/States";
import { formatBytes } from "@/lib/format";
import { scanApi } from "@/lib/scanApi";
import type { TreeNode } from "@/lib/types";
import { categoryColor } from "./categoryColor";

const W = 960;
const H = 520;

interface Props {
  root: string;
}

/** Squarified treemap of one folder level; click a folder to descend. */
export function Treemap({ root }: Props) {
  const { t, i18n } = useTranslation();
  const [stack, setStack] = useState<string[]>([root]);
  const [node, setNode] = useState<TreeNode | null>(null);
  const current = stack[stack.length - 1] ?? root;

  useEffect(() => {
    let live = true;
    scanApi.subtree(current, 1, 60).then((n) => live && setNode(n)).catch(() => live && setNode(null));
    return () => {
      live = false;
    };
  }, [current]);

  const cells = useMemo(() => {
    if (!node) return [];
    // Only leaves carry a value; a node with children gets the sum of them.
    const h = hierarchy<TreeNode>(node, (d) => d.children).sum((d) => (d.children.length ? 0 : d.bytes));
    const laid = treemap<TreeNode>().size([W, H]).paddingInner(2).tile(treemapSquarify)(h) as HierarchyRectangularNode<TreeNode>;
    return laid.leaves().filter((l) => l.x1 - l.x0 > 0 && l.y1 - l.y0 > 0);
  }, [node]);

  if (!node) return <LoadingState />;

  return (
    <div className="grid gap-3">
      <div className="flex items-center justify-between gap-3">
        <p className="path truncate text-fg-muted" title={current}>
          {current}
        </p>
        <Button variant="outline" size="sm" disabled={stack.length <= 1} onClick={() => setStack((s) => s.slice(0, -1))}>
          <ArrowUp aria-hidden className="h-4 w-4" />
          {t("scan.map_up")}
        </Button>
      </div>
      <svg viewBox={`0 0 ${W} ${H}`} className="w-full rounded-md bg-surface-muted" role="group" aria-label={t("scan.tab_map")}>
        {cells.map((c) => {
          const d = c.data;
          const w = c.x1 - c.x0;
          const hgt = c.y1 - c.y0;
          const isOther = d.name === "";
          const label = isOther ? t("scan.map_other", { count: d.files }) : d.name;
          const fill = d.is_dir ? "var(--accent)" : d.category ? categoryColor(d.category) : "var(--cat-other)";
          const clickable = d.is_dir && !isOther;
          return (
            <g
              key={d.path}
              transform={`translate(${c.x0},${c.y0})`}
              role={clickable ? "button" : undefined}
              tabIndex={clickable ? 0 : undefined}
              aria-label={`${label}, ${formatBytes(d.bytes, i18n.language)}`}
              onClick={clickable ? () => setStack((s) => [...s, d.path]) : undefined}
              onKeyDown={clickable ? (e) => e.key === "Enter" && setStack((s) => [...s, d.path]) : undefined}
              className={clickable ? "cursor-pointer outline-none focus-visible:[&>rect]:stroke-focus" : undefined}
            >
              <rect width={w} height={hgt} rx={3} fill={fill} fillOpacity={d.is_dir ? 0.85 : 0.75} stroke="var(--surface)" strokeWidth={0} />
              {w > 70 && hgt > 30 && (
                <text x={8} y={18} fill="#fff" fontSize={13} fontWeight={500} style={{ pointerEvents: "none" }}>
                  <tspan>{label.length > w / 7 ? `${label.slice(0, Math.max(3, Math.floor(w / 7) - 1))}…` : label}</tspan>
                  {hgt > 48 && (
                    <tspan x={8} dy={18} fontWeight={400} fontSize={12} opacity={0.9} className="num">
                      {formatBytes(d.bytes, i18n.language)}
                    </tspan>
                  )}
                </text>
              )}
            </g>
          );
        })}
      </svg>
      <p className="text-sm text-fg-muted">{t("scan.map_hint")}</p>
    </div>
  );
}
