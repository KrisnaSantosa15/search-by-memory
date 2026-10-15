import type { ButtonHTMLAttributes } from "react";

const VARIANTS = {
  /** The one main action of a view. */
  primary:
    "border-accent bg-accent text-accent-ink shadow-xs inset-shadow-[0_1px_rgb(255_255_255/16%)] hover:bg-accent-hover hover:border-accent-hover",
  outline: "border-line bg-panel/40 hover:bg-hover",
  ghost: "border-transparent text-muted hover:bg-hover hover:text-ink",
  /** Erases or removes: tinted, never a solid red. */
  danger: "border-transparent bg-danger/15 text-danger hover:bg-danger/25",
};

const SIZES = {
  sm: "h-7 gap-1.5 px-2.5",
  md: "h-8 gap-1.5 px-3",
  lg: "h-9 gap-2 px-4",
};

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: keyof typeof VARIANTS;
  size?: keyof typeof SIZES;
};

/** A button in the app's style; it submits a form only when `type="submit"`. */
export function Button({ variant = "outline", size = "md", className = "", ...props }: Props) {
  return (
    <button
      type="button"
      {...props}
      className={`inline-flex shrink-0 items-center justify-center rounded-lg border text-sm font-medium whitespace-nowrap transition-colors disabled:pointer-events-none disabled:opacity-50 [&_svg]:size-4 [&_svg]:shrink-0 ${VARIANTS[variant]} ${SIZES[size]} ${className}`}
    />
  );
}
