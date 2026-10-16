import { cn } from "@/lib/utils";
import { Check } from "lucide-react";

interface Option<T extends string> {
  value: T;
  label: string;
  hint?: string;
}

interface Props<T extends string> {
  name: string;
  value: T;
  options: Option<T>[];
  onChange: (value: T) => void;
}

/** Accessible radio-card group with full keyboard support. */
export function ChoiceGroup<T extends string>({ name, value, options, onChange }: Props<T>) {
  return (
    <div role="radiogroup" aria-label={name} className="grid gap-3">
      {options.map((o) => {
        const selected = o.value === value;
        return (
          <label
            key={o.value}
            className={cn(
              "flex cursor-pointer items-start gap-3 rounded-xl border p-4 transition-colors",
              selected ? "border-accent bg-surface" : "border-border bg-surface hover:bg-surface-muted",
            )}
          >
            <input
              type="radio"
              name={name}
              value={o.value}
              checked={selected}
              onChange={() => onChange(o.value)}
              className="sr-only"
            />
            <span
              aria-hidden
              className={cn(
                "mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-full border",
                selected ? "border-accent bg-accent text-accent-fg" : "border-border",
              )}
            >
              {selected && <Check className="h-3.5 w-3.5" />}
            </span>
            <span>
              <span className="block font-medium">{o.label}</span>
              {o.hint && <span className="block text-sm text-fg-muted">{o.hint}</span>}
            </span>
          </label>
        );
      })}
    </div>
  );
}
