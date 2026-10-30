import { useEffect } from 'react';
import { AppShell } from '@/app/AppShell';
import { bootstrap } from '@/app/bootstrap';
import { EmptyState } from '@/app/EmptyState';
import { useAppIcon } from '@/app/useAppIcon';
import { useDocumentSettings } from '@/app/useDocumentSettings';
import { useShortcuts } from '@/app/useShortcuts';
import { CalibrationView } from '@/features/calibration/CalibrationView';
import { ControllerStrip } from '@/features/common/ControllerStrip';
import { MultiView } from '@/features/common/MultiView';
import { HistoryView } from '@/features/history/HistoryView';
import { LatencyView } from '@/features/latency/LatencyView';
import { OverlayView } from '@/features/overlay/OverlayView';
import { SensorsView } from '@/features/sensors/SensorsView';
import { TestView } from '@/features/test/TestView';
import { useT } from '@/i18n';
import { useApp, useSelectedController } from '@/state/app';
import { useSettings } from '@/state/settings';
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
      case 'calibration':
        return c && <CalibrationView key={c.id} c={c} />;
      case 'history':
        return <HistoryView />;
      case 'sensors':
        return c && <SensorsView key={c.id} c={c} />;
      case 'overlay':
        return <OverlayView />;
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
  useAppIcon();
  useShortcuts();
  useEffect(() => {
    // On narrow windows the raw panel is a drawer over the content: start closed.
    if (window.innerWidth < 1180) useSettings.getState().set({ showRawPanel: false });
    void bootstrap();
  }, []);
  return (
    <AppShell>
      <MainContent />
    </AppShell>
  );
}
