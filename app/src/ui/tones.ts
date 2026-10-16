/** Tints for icon tiles, as VoiceStudio gives each destination its own colour. */
export const TONES = {
  sky: "bg-sky-100 text-sky-700 ring-sky-300/60 dark:bg-sky-400/15 dark:text-sky-300 dark:ring-sky-300/30",
  violet:
    "bg-violet-100 text-violet-700 ring-violet-300/60 dark:bg-violet-400/15 dark:text-violet-300 dark:ring-violet-300/30",
  teal: "bg-teal-100 text-teal-700 ring-teal-300/60 dark:bg-teal-400/15 dark:text-teal-300 dark:ring-teal-300/30",
  rose: "bg-rose-100 text-rose-700 ring-rose-300/60 dark:bg-rose-400/15 dark:text-rose-300 dark:ring-rose-300/30",
  amber:
    "bg-amber-100 text-amber-800 ring-amber-300/60 dark:bg-amber-400/15 dark:text-amber-200 dark:ring-amber-300/30",
  fuchsia:
    "bg-fuchsia-100 text-fuchsia-700 ring-fuchsia-300/60 dark:bg-fuchsia-400/15 dark:text-fuchsia-300 dark:ring-fuchsia-300/30",
} as const;

export type Tone = keyof typeof TONES;
