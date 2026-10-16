/**
 * The app's mark: a magnifier over a fading trail of memory, on a rose to
 * violet tile, drawn inline so it stays sharp at every size.
 */
export function BrandMark({ className = "size-6" }: { className?: string }) {
  return (
    <svg viewBox="0 0 32 32" aria-hidden className={className}>
      <defs>
        <linearGradient id="brand-tile" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#f19bb0" />
          <stop offset="0.55" stopColor="#d3869b" />
          <stop offset="1" stopColor="#8f6fd8" />
        </linearGradient>
      </defs>
      <rect width="32" height="32" rx="9" fill="url(#brand-tile)" />
      <rect width="32" height="32" rx="9" fill="none" stroke="white" strokeOpacity="0.22" />
      {/* The trail of what is remembered: three dots, fading into the past. */}
      <circle cx="7.5" cy="23.5" r="1.4" fill="white" fillOpacity="0.45" />
      <circle cx="10.6" cy="20.4" r="1.2" fill="white" fillOpacity="0.65" />
      <circle cx="7.6" cy="17.6" r="1" fill="white" fillOpacity="0.35" />
      <circle cx="18" cy="13.5" r="6" fill="none" stroke="white" strokeWidth="2.6" />
      <path d="M22.4 18l4.1 4.1" stroke="white" strokeWidth="2.8" strokeLinecap="round" />
      <path
        d="M18 10.6l.8 1.9 1.9.8-1.9.8-.8 1.9-.8-1.9-1.9-.8 1.9-.8z"
        fill="white"
      />
    </svg>
  );
}
