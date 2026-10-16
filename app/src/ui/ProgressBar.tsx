type Props = {
  /** What the bar measures, for screen readers. */
  label: string;
  /** How far along, from 0 to 1. */
  value: number;
};

/** A thin bar in the action colour, as VoiceStudio draws downloads. */
export function ProgressBar({ label, value }: Props) {
  const percent = Math.round(Math.min(Math.max(value, 0), 1) * 100);
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={percent}
      className="h-1.5 overflow-hidden rounded-full bg-hover"
    >
      <div
        className="h-full rounded-full bg-accent transition-[width] duration-300"
        style={{ width: `${percent}%` }}
      />
    </div>
  );
}
