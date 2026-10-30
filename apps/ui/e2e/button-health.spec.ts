import { readFile, stat } from 'node:fs/promises';
import { expect, test, type Page } from '@playwright/test';

async function startDemo(page: Page) {
  await page.goto('/');
  await page.getByTestId('empty-demo').click();
  await expect(page.getByTestId('controller-item')).toHaveCount(1);
}

test('button health flags the demo R1 and keeps L1 healthy', async ({ page }) => {
  await startDemo(page);
  await page.getByRole('radio', { name: 'Button health' }).click();
  // The demo is read like HID: no low-precision warning.
  await expect(page.getByTestId('health-precision')).toContainText('Direct reading (HID)');
  await page.getByRole('button', { name: 'Shoulders and triggers' }).click();
  await page.getByRole('checkbox', { name: 'L2' }).uncheck();
  await page.getByRole('checkbox', { name: 'R2' }).uncheck();
  await page.getByTestId('health-start').click();
  await expect(page.getByRole('heading', { name: /Press L1 20 times/ })).toBeVisible();
  await expect(page.getByTestId('health-counter')).toContainText(/[1-9]\d? of 20/);

  const result = page.getByTestId('health-result');
  await expect(result).toBeVisible({ timeout: 60_000 });
  await expect(result).toContainText('Good: 1 · Attention: 0 · Bad: 1');
  await expect(result).toContainText('one report every 4 ms (HID)');
  await expect(page.getByTestId('health-card-4')).toHaveAttribute('data-grade', 'good');
  const r1 = page.getByTestId('health-card-5');
  await expect(r1).toHaveAttribute('data-grade', 'bad');
  await expect(r1).toContainText('3 press(es) never reached the app.');
  await expect(r1).toContainText('with chatter');
  await expect(r1).toContainText('Replace the switch');

  await page.getByTestId('save-history').click();
  await expect(page.getByText('Session saved to history')).toBeVisible();

  // History and the three report formats.
  await page.getByTestId('tab-history').click();
  await expect(page.getByTestId('history-table')).toContainText('Button health');
  await page.getByRole('checkbox', { name: /Select session/ }).check();
  for (const [label, ext] of [
    ['JSON data', 'json'],
    ['PNG image', 'png'],
    ['PDF document', 'pdf'],
  ] as const) {
    await page.getByTestId('history-report').click();
    await page.getByRole('radio', { name: label }).click();
    const download = page.waitForEvent('download');
    await page.getByTestId('report-generate').click();
    const file = await download;
    expect(file.suggestedFilename()).toMatch(new RegExp(`\\.${ext}$`));
    const path = await file.path();
    if (ext === 'json') {
      const json = JSON.parse(await readFile(path, 'utf8'));
      const session = json.sessions[0];
      expect(session.kind).toBe('buttonHealth');
      const r1 = session.payload.report.buttons.find((b: { button: number }) => b.button === 5);
      expect(r1.grade).toBe('bad');
      expect(r1.advice[0]).toBe('replaceSwitch');
    } else {
      expect((await stat(path)).size).toBeGreaterThan(10_000);
    }
  }
});

test('the guided test hands skipped buttons to the health check', async ({ page }) => {
  await startDemo(page);
  await page.getByRole('radio', { name: 'Button test' }).click();
  await page.getByTestId('guided-start').click();
  // Skip the first prompt before the demo pad presses it.
  await page.getByRole('button', { name: 'Skip' }).click();
  await expect(page.getByTestId('guided-result')).toContainText('1 skipped', { timeout: 60_000 });
  await page.getByTestId('guided-check-health').click();
  await expect(page.getByText('already ticked')).toBeVisible();
  await expect(page.getByRole('checkbox', { name: '✕' })).toBeChecked();
  await expect(page.getByRole('checkbox', { name: '○' })).not.toBeChecked();
});
