/** Stands in for the launcher until it is built: indexing goes on meanwhile. */
export function Launcher() {
  return (
    <section className="flex flex-col gap-1">
      <h1 className="text-xl font-semibold">Search by Memory</h1>
      <p className="text-sm text-muted">
        Searching is on its way in the next version. Your files keep becoming searchable in the
        background.
      </p>
    </section>
  );
}
