/** A path that, when too long, loses its start: the folder and name stay readable. */
export function Path({ path }: { path: string }) {
  return (
    <span className="min-w-0 truncate text-left [direction:rtl]">
      <bdi>{path}</bdi>
    </span>
  );
}
