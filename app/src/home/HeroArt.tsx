/**
 * Home's banner art: files floating like memories, a thread of light linking
 * them to the magnifier that finds them. Decorative only.
 */
export function HeroArt() {
  return (
    <svg
      aria-hidden
      viewBox="0 0 360 150"
      fill="none"
      className="pointer-events-none absolute -top-6 right-0 -z-10 h-[140px] w-[380px] opacity-70 [mask-image:linear-gradient(90deg,transparent,black_45%)]"
    >
      <defs>
        <linearGradient id="hero-thread" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0" stopColor="var(--color-accent)" stopOpacity="0" />
          <stop offset="1" stopColor="var(--color-accent)" stopOpacity="0.9" />
        </linearGradient>
        <radialGradient id="hero-glow">
          <stop offset="0" stopColor="var(--color-accent)" stopOpacity="0.35" />
          <stop offset="1" stopColor="var(--color-accent)" stopOpacity="0" />
        </radialGradient>
      </defs>
      <circle cx="292" cy="70" r="70" fill="url(#hero-glow)" />
      {/* The thread from each remembered file to the magnifier. */}
      <path
        d="M62 38 C 130 30, 180 60, 270 66 M92 112 C 150 120, 200 90, 270 74 M170 22 C 210 26, 240 46, 272 62"
        stroke="url(#hero-thread)"
        strokeWidth="1.2"
        strokeDasharray="2 5"
        strokeLinecap="round"
      />
      <g stroke="var(--color-muted)" strokeOpacity="0.55" strokeWidth="1.2">
        <FileGlyph x={40} y={22} rotate={-8} />
        <FileGlyph x={76} y={96} rotate={6} />
        <FileGlyph x={152} y={6} rotate={-3} />
        <FileGlyph x={196} y={104} rotate={10} small />
      </g>
      <g stroke="var(--color-accent)" strokeWidth="3" strokeLinecap="round">
        <circle cx="292" cy="66" r="22" />
        <path d="M308 82 l18 18" strokeWidth="5" />
      </g>
      <path
        d="M292 54 l2.6 6.4 6.4 2.6 -6.4 2.6 -2.6 6.4 -2.6 -6.4 -6.4 -2.6 6.4 -2.6z"
        fill="var(--color-accent)"
      />
    </svg>
  );
}

function FileGlyph({ x, y, rotate, small = false }: { x: number; y: number; rotate: number; small?: boolean }) {
  const scale = small ? 0.8 : 1;
  return (
    <g transform={`translate(${x} ${y}) rotate(${rotate}) scale(${scale})`}>
      <path d="M0 3 a3 3 0 0 1 3 -3 h16 l9 9 v25 a3 3 0 0 1 -3 3 h-22 a3 3 0 0 1 -3 -3z" />
      <path d="M19 0 v9 h9" />
      <path d="M6 17 h14 M6 23 h10 M6 29 h12" strokeOpacity="0.6" />
    </g>
  );
}
