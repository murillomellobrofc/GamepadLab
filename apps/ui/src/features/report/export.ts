import { toPng } from 'html-to-image';
import type { HistorySession } from '@/api/types';
import { saveBase64, saveText, stamp } from '@/lib/files';

export type ReportFormat = 'png' | 'pdf' | 'json';

export function reportJson(sessions: HistorySession[], version: string): string {
  return JSON.stringify(
    {
      format: 'gamepadlab-report',
      version: 1,
      appVersion: version,
      generatedAt: new Date().toISOString(),
      sessions,
    },
    null,
    2,
  );
}

async function nodeToPng(node: HTMLElement): Promise<string> {
  // Two frames so canvases inside the sheet have drawn.
  await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
  return toPng(node, { pixelRatio: 2, backgroundColor: '#ffffff', cacheBust: true });
}

/** Renders and saves the report; returns false when the user cancelled. */
export async function exportReport(
  format: ReportFormat,
  sessions: HistorySession[],
  node: HTMLElement | null,
  version: string,
): Promise<boolean> {
  const base = `gamepadlab-report-${stamp()}`;
  if (format === 'json') {
    return saveText(`${base}.json`, reportJson(sessions, version), [
      { name: 'JSON', extensions: ['json'] },
    ]);
  }
  if (!node) throw new Error('report not rendered');
  const dataUrl = await nodeToPng(node);
  if (format === 'png') {
    return saveBase64(
      `${base}.png`,
      dataUrl.split(',')[1],
      [{ name: 'PNG', extensions: ['png'] }],
      'image/png',
    );
  }
  const { jsPDF } = await import('jspdf');
  const pdf = new jsPDF({ orientation: 'portrait', unit: 'pt', format: 'a4' });
  const pageW = pdf.internal.pageSize.getWidth();
  const pageH = pdf.internal.pageSize.getHeight();
  const img = await new Promise<HTMLImageElement>((resolve, reject) => {
    const i = new Image();
    i.onload = () => resolve(i);
    i.onerror = reject;
    i.src = dataUrl;
  });
  const scale = pageW / img.width;
  const sliceH = Math.floor(pageH / scale);
  const canvas = document.createElement('canvas');
  canvas.width = img.width;
  for (let y = 0, page = 0; y < img.height; y += sliceH, page++) {
    const h = Math.min(sliceH, img.height - y);
    canvas.height = h;
    const ctx = canvas.getContext('2d')!;
    ctx.fillStyle = '#fff';
    ctx.fillRect(0, 0, canvas.width, h);
    ctx.drawImage(img, 0, y, img.width, h, 0, 0, img.width, h);
    if (page > 0) pdf.addPage();
    pdf.addImage(canvas.toDataURL('image/png'), 'PNG', 0, 0, pageW, h * scale);
  }
  pdf.setProperties({ title: 'GamepadLab report', creator: `GamepadLab ${version}` });
  const b64 = pdf.output('datauristring').split(',')[1];
  return saveBase64(`${base}.pdf`, b64, [{ name: 'PDF', extensions: ['pdf'] }], 'application/pdf');
}
