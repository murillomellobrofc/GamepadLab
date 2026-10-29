import { useEffect } from 'react';
import { AppShell } from '@/app/AppShell';
import { bootstrap } from '@/app/bootstrap';
import { EmptyState } from '@/app/EmptyState';
import { useDocumentSettings } from '@/app/useDocumentSettings';
import { useShortcuts } from '@/app/useShortcuts';
import { ControllerStrip } from '@/features/common/ControllerStrip';
import { MultiView } from '@/features/common/MultiView';
import { LatencyView } from '@/features/latency/LatencyView';
import { TestView } from '@/features/test/TestView';
import { useT } from '@/i18n';
import { useApp, useSelectedController } from '@/state/app';
import ui from '@/styles/ui.module.css';

function MainContent() {
  const t = useT();
  const tab = useApp((s) => s.tab);
  const c = useSelectedController();
  const multi = useApp((s) => s.multiView);
  if (!c && tab !== 'history' && tab !== 'overlay') return <EmptyState />;
  if (multi && (tab === 'test' || tab === 'latency'))
    return (
      <>
        <ControllerStrip />
        <MultiView />
      </>
    );
  const body = (() => {
    switch (tab) {
      case 'test':
        return c && <TestView key={c.id} c={c} />;
      case 'latency':
        return c && <LatencyView key={c.id} c={c} />;
      default:
        return <p className={ui.muted}>{t('app.common.loading')}</p>;
    }
  })();
  return (
    <>
      <ControllerStrip />
      {body}
    </>
  );
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
