import { useState } from 'react';
import { Icon } from './icons';
import ui from '@/styles/ui.module.css';

/** "Save to history" button that turns into a check once saved. */
export function SaveToHistory({
  label,
  onSave,
}: {
  label: string;
  onSave: () => Promise<number | null>;
}) {
  const [saved, setSaved] = useState(false);
  return (
    <button
      type="button"
      className={ui.btn}
      disabled={saved}
      onClick={async () => setSaved((await onSave()) != null)}
      data-testid="save-history"
    >
      <Icon name={saved ? 'check' : 'history'} size={16} /> {label}
    </button>
  );
}
