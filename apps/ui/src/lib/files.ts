import { api } from '@/api/client';
import { isTauri } from '@/api/transport';

export interface FileFilter {
  name: string;
  extensions: string[];
}

function download(name: string, blob: Blob) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  document.body.appendChild(a);
  a.click();
  a.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 2000);
}

/** Native "save as" in the desktop app, a download in the browser. */
export async function saveText(
  name: string,
  contents: string,
  filters: FileFilter[],
  mime = 'application/json',
) {
  if (!isTauri()) {
    download(name, new Blob([contents], { type: mime }));
    return true;
  }
  const { save } = await import('@tauri-apps/plugin-dialog');
  const path = await save({ defaultPath: name, filters });
  if (!path) return false;
  await api.saveTextFile(path, contents);
  return true;
}

export async function saveBase64(
  name: string,
  base64: string,
  filters: FileFilter[],
  mime: string,
) {
  if (!isTauri()) {
    const bin = atob(base64);
    const bytes = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
    download(name, new Blob([bytes], { type: mime }));
    return true;
  }
  const { save } = await import('@tauri-apps/plugin-dialog');
  const path = await save({ defaultPath: name, filters });
  if (!path) return false;
  await api.saveBinaryFile(path, base64);
  return true;
}

/** Native "open" dialog (or a file input in the browser). Returns the text. */
export async function openText(filters: FileFilter[]): Promise<string | null> {
  if (!isTauri()) {
    return new Promise((resolve) => {
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = filters.flatMap((f) => f.extensions.map((e) => `.${e}`)).join(',');
      input.onchange = async () => resolve(input.files?.[0] ? await input.files[0].text() : null);
      input.click();
    });
  }
  const { open } = await import('@tauri-apps/plugin-dialog');
  const path = await open({ multiple: false, directory: false, filters });
  if (!path || Array.isArray(path)) return null;
  return api.readTextFile(path);
}

export function stamp(d = new Date()): string {
  const p = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}`;
}
