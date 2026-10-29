import { expect, test, type Page } from '@playwright/test';

async function startDemo(page: Page) {
  await page.goto('/');
  await page.getByTestId('empty-demo').click();
  await expect(page.getByTestId('controller-item')).toHaveCount(1);
}

async function press(page: Page, button: number) {
  await page.evaluate((b) => {
    const m = (
      window as unknown as {
        __gamepadlabMock: {
          sim: { setScenario: (s: unknown, t: number) => void };
          now: () => number;
        };
      }
    ).__gamepadlabMock;
    m.sim.setScenario({ pressButton: b }, m.now());
  }, button);
  await page.waitForTimeout(900);
}

test('save a session and export reports in every format', async ({ page }) => {
  await startDemo(page);
  await page.getByRole('radio', { name: 'Ghosting' }).click();
  await page.getByTestId('ghost-start').click();
  await expect(page.getByTestId('ghost-result')).toBeVisible();
  await page.getByTestId('save-history').click();
  await expect(page.getByText('Session saved to history')).toBeVisible();

  await page.getByTestId('tab-history').click();
  await expect(page.getByTestId('history-table')).toContainText('Ghosting');
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
    expect(file.suggestedFilename()).toMatch(new RegExp(`^gamepadlab-report-.*\\.${ext}$`));
    const path = await file.path();
    expect(path).toBeTruthy();
  }
});

test('sensors tab shows orientation, IMU and touchpad', async ({ page }) => {
  await startDemo(page);
  await page.getByTestId('tab-sensors').click();
  await expect(page.getByRole('img', { name: 'Gyroscope' })).toBeVisible();
  await expect(page.getByRole('img', { name: 'Touchpad' })).toBeVisible();
  await page.getByRole('button', { name: 'Reset orientation' }).click();
  await expect(page.getByText('Pitch')).toBeVisible();
});

test('overlay settings drive the preview', async ({ page }) => {
  await startDemo(page);
  await page.getByTestId('tab-overlay').click();
  await expect(page.getByRole('img', { name: 'DualSense (demo)' })).toBeVisible();
  await page.getByRole('radio', { name: 'Chroma green' }).click();
  const bg = await page
    .getByRole('img', { name: 'DualSense (demo)' })
    .evaluate(
      (el) => getComputedStyle(el.closest('[style*="--ov-scale"]') as HTMLElement).backgroundColor,
    );
  expect(bg).toBe('rgb(0, 177, 64)');
});

test('the controller itself can drive the app', async ({ page }) => {
  await startDemo(page);
  await press(page, 16); // Guide/PS toggles navigation
  await expect(page.getByText(/D-pad\/stick move focus/)).toBeVisible();
  await press(page, 5); // RB → next tab
  await expect(page.getByTestId('tab-calibration')).toHaveAttribute('aria-selected', 'true');
  await press(page, 16);
  await expect(page.getByText(/D-pad\/stick move focus/)).toBeHidden();
});
