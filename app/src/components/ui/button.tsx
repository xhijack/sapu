import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 rounded-full font-semibold transition-[background,box-shadow,transform] duration-150 disabled:pointer-events-none disabled:opacity-50 whitespace-nowrap active:scale-[0.98] motion-reduce:transform-none",
  {
    variants: {
      variant: {
        primary: "bg-accent text-accent-fg shadow-[var(--shadow-glow)] hover:bg-accent-strong",
        secondary: "bg-surface-muted text-fg hover:bg-border-strong",
        ghost: "text-fg-muted hover:bg-surface-muted hover:text-fg",
        outline: "border border-border-strong bg-transparent text-fg hover:bg-surface-muted",
      },
      size: {
        sm: "h-8 px-3.5 text-sm",
        md: "h-10 px-5 text-sm",
        lg: "h-13 px-7 text-base",
      },
    },
    defaultVariants: { variant: "primary", size: "md" },
  },
);

export interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement>, VariantProps<typeof buttonVariants> {}

export function Button({ className, variant, size, type = "button", ...props }: ButtonProps) {
  return <button type={type} className={cn(buttonVariants({ variant, size }), className)} {...props} />;
}
