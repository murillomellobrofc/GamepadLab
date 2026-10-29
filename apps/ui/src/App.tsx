import { useEffect } from 'react';
import { AppShell } from '@/app/AppShell';
import { bootstrap } from '@/app/bootstrap';
import { EmptyState } from '@/app/EmptyState';
import { useDocumentSettings } from '@/app/useDocumentSettings';
import { useShortcuts } from '@/app/useShortcuts';
import { useApp } from '@/state/app';
import { useT } from '@/i18n';

function MainContent() {
  const t = useT();
  const hasControllers = useApp((s) => s.controllers.length > 0);
  const tab = useApp((s) => s.tab);
  if (!hasControllers && tab !== 'history' && tab !== 'overlay') return <EmptyState />;
  return <p>{t('app.common.loading')}</p>;
}

export default function App() {
  useDocumentSettings();
  useShortcuts();
  useEffect(() => {
    void bootstrap();
  }, []);
  return (
    <AppShell>
      <MainContent />
    </AppShell>
  );
}
