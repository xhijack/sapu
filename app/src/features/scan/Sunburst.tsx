import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { hierarchy, partition, type HierarchyRectangularNode } from "d3-hierarchy";
import { arc as d3arc } from "d3-shape";
import { ArrowUp, FolderOpen, Plus, Trash2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { LoadingState } from "@/components/States";
import { formatBytes } from "@/lib/format";
import { scanApi } from "@/lib/scanApi";
import type { TreeNode } from "@/lib/types";
import { categoryColor } from "./categoryColor";

const SIZE = 440;
const R = SIZE / 2;
const HOLE = 58;
const PALETTE = ["var(--cat-video)", "var(--cat-development)", "var(--cat-photo)", "var(--cat-application)", "var(--cat-document)", "var(--cat-temporary)", "var(--cat-system)", "var(--cat-other)"];

export interface BasketItem {
  path: string;
  bytes: number;
  is_dir: boolean;
}

interface Props {
  root: string;
  onTrash: (paths: string[]) => void;
}

type Node = HierarchyRectangularNode<TreeNode>;

/** DaisyDisk-style rings: drill in, pick, collect in a basket, remove once. */
export function Sunburst({ root, onTrash }: Props) {
  const { t, i18n } = useTranslation();
  const [stack, setStack] = useState<string[]>([root]);
  const [tree, setTree] = useState<TreeNode | null>(null);
  const [selected, setSelected] = useState<TreeNode | null>(null);
  const [basket, setBasket] = useState<Map<string, BasketItem>>(new Map());
  const current = stack[stack.length - 1] ?? root;
  const lang = i18n.language;

  useEffect(() => {
    let live = true;
    scanApi
      .subtree(current, 3, 24)
      .then((n) => live && setTree(n))
      .catch(() => live && setTree(null));
    return () => {
      live = false;
    };
  }, [current]);

  const nodes = useMemo(() => {
    if (!tree) return [];
    const h = hierarchy<TreeNode>(tree, (d) => d.children).sum((d) => (d.children.length ? 0 : d.bytes)).sort((a, b) => (b.value ?? 0) - (a.value ?? 0));
    partition<TreeNode>().size([2 * Math.PI, R - HOLE])(h);
    return (h.descendants() as Node[]).filter((n) => n.depth > 0 && n.x1 - n.x0 > 0.004);
  }, [tree]);

  const arcGen = d3arc<Node>()
    .startAngle((d) => d.x0)
    .endAngle((d) => d.x1)
    .padAngle(0.004)
    .innerRadius((d) => HOLE + d.y0)
    .outerRadius((d) => HOLE + d.y1 - 2)
    .cornerRadius(2);

  const colorFor = (n: Node) => {
    let top: Node = n;
    while (top.depth > 1 && top.parent) top = top.parent as Node;
    const idx = top.parent ? (top.parent.children ?? []).indexOf(top) : 0;
    const base = n.data.category && !n.data.is_dir ? categoryColor(n.data.category) : PALETTE[idx % PALETTE.length]!;
    const fade = 1 - Math.min(0.45, (n.depth - 1) * 0.15);
    return { fill: base, opacity: fade };
  };

  const add = (d: TreeNode) =>
    setBasket((b) => {
      const n = new Map(b);
      n.set(d.path, { path: d.path, bytes: d.bytes, is_dir: d.is_dir });
      return n;
    });
  const remove = (path: string) =>
    setBasket((b) => {
      const n = new Map(b);
      n.delete(path);
      return n;
    });
  const basketItems = [...basket.values()];
  const basketBytes = basketItems.reduce((a, i) => a + i.bytes, 0);

  return (
    <div className="grid gap-5 lg:grid-cols-[auto_1fr]">
      <div className="grid gap-3">
        <div className="relative" style={{ width: SIZE, height: SIZE }}>
          {tree ? (
            <svg viewBox={`${-R} ${-R} ${SIZE} ${SIZE}`} width={SIZE} height={SIZE} role="group" aria-label={t("scan.map_sunburst")}>
              {nodes.map((n) => {
                const d = n.data;
                const sel = selected?.path === d.path;
                const inBasket = basket.has(d.path);
                const { fill, opacity } = colorFor(n);
                const isOther = d.name === "";
                return (
                  <path
                    key={d.path}
                    d={arcGen(n) ?? undefined}
                    fill={fill}
                    fillOpacity={inBasket ? 1 : opacity}
                    stroke={sel || inBasket ? "var(--fg)" : "var(--bg)"}
                    strokeWidth={sel ? 2 : inBasket ? 1.5 : 1}
                    role={isOther ? undefined : "button"}
                    tabIndex={isOther ? undefined : 0}
                    aria-label={`${isOther ? t("scan.map_other", { count: d.files }) : d.name}, ${formatBytes(d.bytes, lang)}`}
                    onClick={() => !isOther && setSelected(d)}
                    onDoubleClick={() => d.is_dir && !isOther && setStack((s) => [...s, d.path])}
                    onKeyDown={(e) => {
                      if (isOther) return;
                      if (e.key === "Enter") {
                        if (d.is_dir) setStack((s) => [...s, d.path]);
                        else setSelected(d);
                      }
                      if (e.key === " ") {
                        e.preventDefault();
                        setSelected(d);
                      }
                    }}
                    className="cursor-pointer outline-none transition-[fill-opacity] hover:fill-opacity-100 focus-visible:stroke-focus"
                  >
                    <title>{`${d.name || t("scan.map_other", { count: d.files })} · ${formatBytes(d.bytes, lang)}`}</title>
                  </path>
                );
              })}
            </svg>
          ) : (
            <div className="grid h-full place-items-center">
              <LoadingState />
            </div>
          )}
          <button
            type="button"
            onClick={() => setStack((s) => (s.length > 1 ? s.slice(0, -1) : s))}
            disabled={stack.length <= 1}
            aria-label={t("scan.sun_up")}
            className="absolute left-1/2 top-1/2 grid h-[112px] w-[112px] -translate-x-1/2 -translate-y-1/2 place-items-center rounded-full bg-surface-solid text-center shadow-[var(--shadow-1)] disabled:cursor-default"
          >
            <span className="grid gap-0.5 px-2">
              <span className="truncate text-xs font-medium" title={current}>
                {current.split("/").filter(Boolean).pop() || current}
              </span>
              <span className="display num text-sm">{tree ? formatBytes(tree.bytes, lang) : ""}</span>
              {stack.length > 1 && <ArrowUp aria-hidden className="mx-auto h-3.5 w-3.5 text-fg-muted" />}
            </span>
          </button>
        </div>
        <p className="max-w-[440px] text-xs text-fg-muted">{t("scan.sun_hint")}</p>
      </div>

      <div className="grid content-start gap-4">
        {selected && (
          <div className="card-solid grid gap-3 p-4">
            <div className="min-w-0">
              <p className="truncate font-medium" title={selected.path}>
                {selected.name}
              </p>
              <p className="path truncate text-fg-muted" title={selected.path}>
                {selected.path}
              </p>
              <p className="display num text-xl">{formatBytes(selected.bytes, lang)}</p>
              {selected.is_dir && <p className="num text-xs text-fg-muted">{t("scan.files_count", { count: selected.files })}</p>}
            </div>
            <div className="flex flex-wrap gap-2">
              {selected.is_dir && (
                <Button variant="secondary" size="sm" onClick={() => setStack((s) => [...s, selected.path])}>
                  {t("scan.sun_enter")}
                </Button>
              )}
              <Button variant="outline" size="sm" onClick={() => void scanApi.reveal(selected.path)}>
                <FolderOpen aria-hidden className="h-4 w-4" />
                {t("scan.action_reveal")}
              </Button>
              {basket.has(selected.path) ? (
                <Button variant="ghost" size="sm" onClick={() => remove(selected.path)}>
                  <X aria-hidden className="h-4 w-4" />
                  {t("scan.basket_remove")}
                </Button>
              ) : (
                <Button size="sm" onClick={() => add(selected)}>
                  <Plus aria-hidden className="h-4 w-4" />
                  {t("scan.basket_add")}
                </Button>
              )}
            </div>
          </div>
        )}
        <div className="card-solid grid gap-3 p-4">
          <div className="flex items-center justify-between">
            <h3 className="font-semibold">{t("scan.basket")}</h3>
            {basketItems.length > 0 && <span className="num text-sm text-fg-muted">{t("scan.basket_total", { count: basketItems.length, size: formatBytes(basketBytes, lang) })}</span>}
          </div>
          {basketItems.length === 0 ? (
            <p className="text-sm text-fg-faint">{t("scan.basket_empty")}</p>
          ) : (
            <>
              <ul className="max-h-56 divide-y divide-border overflow-y-auto text-sm">
                {basketItems.map((i) => (
                  <li key={i.path} className="flex items-center gap-2 py-1.5">
                    <span className="path min-w-0 flex-1 truncate" title={i.path}>
                      {i.path}
                    </span>
                    <span className="num shrink-0 text-xs text-fg-muted">{formatBytes(i.bytes, lang)}</span>
                    <button type="button" aria-label={t("scan.basket_remove")} onClick={() => remove(i.path)} className="text-fg-muted hover:text-fg">
                      <X className="h-3.5 w-3.5" />
                    </button>
                  </li>
                ))}
              </ul>
              <Button onClick={() => onTrash(basketItems.map((i) => i.path))}>
                <Trash2 aria-hidden className="h-4 w-4" />
                {t("scan.basket_trash")}
              </Button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
