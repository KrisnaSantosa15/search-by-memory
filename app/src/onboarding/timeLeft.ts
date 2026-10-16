/** Seconds still to go, in plain words: `About 1 hour 20 minutes left`. */
export function timeLeft(seconds: number | null): string {
  if (seconds === null) return "Working out how long this takes";
  if (seconds < 60) return "Less than a minute left";
  const minutes = Math.round(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const parts = [
    hours > 0 && plural(hours, "hour"),
    minutes % 60 > 0 && plural(minutes % 60, "minute"),
  ].filter(Boolean);
  return `About ${parts.join(" ")} left`;
}

function plural(count: number, unit: string): string {
  return `${count} ${unit}${count === 1 ? "" : "s"}`;
}
