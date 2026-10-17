interface Props {
  id: string;
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (v: string) => void;
}

export function FilterSelect({ id, label, value, options, onChange }: Props) {
  return (
    <label className="grid gap-1 text-sm">
      <span className="text-fg-muted">{label}</span>
      <select id={id} value={value} onChange={(e) => onChange(e.target.value)} className="h-9 rounded-md border border-border bg-surface px-2">
        {options.map((o) => (
          <option key={o.value} value={o.value}>
            {o.label}
          </option>
        ))}
      </select>
    </label>
  );
}
