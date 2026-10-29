import { api, events } from '@/api/client';
import { getTransport } from '@/api/transport';
import { translate } from '@/i18n';
import { useApp } from '@/state/app';
import { live } from '@/state/live';
import { useSettings } from '@/state/settings';

let started = false;

/** Connects the transport, loads the initial state and wires backend events. */
export async function bootstrap(): Promise<void> {
  if (started) return;
  started = true;
  const tr = await getTransport();
  const app = useApp.getState();
  const tt = (k: Parameters<typeof translate>[1], v?: Record<string, string | number>) =>
    translate(useSettings.getState().locale, k, v);

  let info = null;
  try {
    info = await api.appInfo();
  } catch {
    /* the shell still renders without it */
  }
  app.setReady(tr.kind, info);

  const settings = useSettings.getState();
  void api.setUiRate(settings.uiRate).catch(() => {});
  void api.setLogLevel(settings.logLevel).catch(() => {});

  await events.frame((f) => live.push(f));
  await events.connection(({ controller, connected }) => {
    const s = useApp.getState();
    if (connected) {
      s.upsertController(controller);
      s.toast({ tone: 'ok', title: tt('app.toast.connected', { name: controller.name }) });
      s.announce(tt('app.toast.connected', { name: controller.name }));
    } else {
      s.removeController(controller.id);
      live.drop(controller.id);
      s.toast({
        tone: 'warn',
        title: tt('app.toast.disconnected', { name: controller.name }),
        body: controller.isDemo ? undefined : tt('app.toast.disconnectedBody'),
      });
      s.announce(tt('app.toast.disconnected', { name: controller.name }));
    }
  });
  await events.error((e) => {
    useApp.getState().toast({ tone: 'bad', title: tt('app.toast.error'), body: e.message });
  });
  await events.rumble((f) => useApp.getState().setRumble(f));

  try {
    app.setControllers(await api.listControllers());
  } catch {
    /* empty list */
  }
  try {
    app.setWarnings(await api.platformWarnings());
  } catch {
    /* none */
  }
}
