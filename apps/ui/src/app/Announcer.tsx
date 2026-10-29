import { useApp } from '@/state/app';

/** Screen-reader live region; anything passed to `announce()` is read aloud. */
export function Announcer() {
  const { text, n } = useApp((s) => s.announcement);
  return (
    <div className="sr-only" role="status" aria-live="assertive" aria-atomic="true">
      <span key={n}>{text}</span>
    </div>
  );
}
