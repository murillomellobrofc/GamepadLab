import { expect, test, type Page } from '@playwright/test';

async function startDemo(page: Page) {
  await page.goto('/');
  await expect(page.getByTestId('empty-state')).toBeVisible();
  await page.getByTestId('empty-demo').click();
  await expect(page.getByTestId('controller-item')).toHaveCount(1);
  await expect(page.getByTestId('controller-item')).toContainText('DualSense (demo)');
}

test('empty state explains what to do and offers the demo', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByRole('heading', { name: 'No controller connected' })).toBeVisible();
  await expect(page.getByText(/Tips for/)).toBeVisible();
});

test('live test view, raw panel and min/max reset', async ({ page }) => {
  await startDemo(page);
  await expect(page.getByRole('img', { name: 'DualSense (demo)' })).toBeVisible();
  await expect(page.getByRole('table', { name: 'Axes' })).toContainText('LX');
  await expect(page.getByRole('table', { name: 'Buttons' })).toContainText('Cross');
  await page.getByTestId('raw-reset').click();
  await expect(page.getByText('Minimums and maximums cleared')).toBeVisible();
});

test('guided button test passes every button of the demo pad', async ({ page }) => {
  await startDemo(page);
  await page.getByRole('radio', { name: 'Button test' }).click();
  await page.getByTestId('guided-start').click();
  await expect(page.getByTestId('guided-prompt')).toContainText('Press');
  await expect(page.getByTestId('guided-result')).toContainText('19 passed, 0 failed, 0 skipped', {
    timeout: 60_000,
  });
});

test('ghosting test finds the demo fault', async ({ page }) => {
  await startDemo(page);
  await page.getByRole('radio', { name: 'Ghosting' }).click();
  await page.getByTestId('ghost-start').click();
  await expect(page.getByTestId('ghost-result')).toContainText('Possible ghosting');
});

test('rumble shows on screen', async ({ page }) => {
  await startDemo(page);
  await page.getByRole('radio', { name: 'Rumble' }).click();
  await page.getByTestId('rumble-play').click();
  await expect(page.getByText(/Rumbling: weak 60%, strong 80%/)).toBeVisible();
});

test('calibration wizard end to end, apply and compare', async ({ page }) => {
  await startDemo(page);
  await page.getByTestId('tab-calibration').click();
  await page.getByTestId('wizard-start').click();

  await page.getByTestId('rest-measure').click();
  await expect(page.getByTestId('wizard-rest')).toContainText('Centre measured');
  await page.getByTestId('wizard-next').click();

  await page.getByTestId('rotation-start').click();
  await expect(page.getByTestId('wizard-rotation')).toContainText(/Circle coverage: (9\d|100)%/, {
    timeout: 20_000,
  });
  await page.getByTestId('rotation-finish').click();

  await page.getByTestId('directions-start').click();
  // The wizard moves on by itself once all 8 directions light up.
  await expect(page.getByTestId('wizard-triggers')).toBeVisible({ timeout: 20_000 });

  await page.getByTestId('trigger-rest').click();
  await page.getByTestId('trigger-press').click();
  await page.getByTestId('trigger-done').click();
  await page.getByTestId('trigger-press').click();
  await page.getByTestId('trigger-done').click();

  const report = page.getByTestId('stick-report');
  await expect(report).toBeVisible();
  await expect(report).toContainText('Quality score');
  await expect(report).toContainText('Off-centre (drift)');
  await expect(report).toContainText('Weak sectors');

  await page.getByTestId('apply-suggestion').click();
  await expect(page.getByTestId('compare')).toContainText('Before');
  await expect(page.getByText(/Calibration applied to profile/)).toBeVisible();

  // The calibrated view is now available everywhere.
  await page.getByRole('radio', { name: 'Calibrated' }).click();
  await page.getByTestId('tab-test').click();
  await expect(page.getByRole('radio', { name: 'Calibrated' })).toHaveAttribute(
    'aria-checked',
    'true',
  );
});

test('profiles: create, export and switch', async ({ page }) => {
  await startDemo(page);
  await page.getByTestId('tab-calibration').click();
  await page.getByRole('radio', { name: 'Profiles' }).click();
  await expect(page.getByTestId('profile-item')).toHaveCount(1);
  await page.getByTestId('profile-new').click();
  await expect(page.getByTestId('profile-item')).toHaveCount(2);
  const download = page.waitForEvent('download');
  await page
    .getByTestId('profile-item')
    .first()
    .getByRole('button', { name: 'Export JSON' })
    .click();
  expect((await download).suggestedFilename()).toMatch(/^gamepadlab-.*\.json$/);
  await page.getByTestId('profile-item').first().getByRole('button', { name: 'Activate' }).click();
  await expect(page.getByTestId('profile-item').first()).toContainText('Active');
});

test('latency tab reports the demo polling rate', async ({ page }) => {
  await startDemo(page);
  await page.getByTestId('tab-latency').click();
  await expect(page.getByTestId('latency-live')).toContainText(/2[45]\d\s*Hz/);
});

test('language and theme switch, keyboard tab navigation', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: 'Settings' }).click();
  await page.getByRole('combobox', { name: 'Language' }).selectOption('pt-BR');
  await expect(page.getByRole('heading', { name: 'Configurações' })).toBeVisible();
  await page.getByRole('combobox', { name: 'Idioma' }).selectOption('es');
  await expect(page.getByRole('heading', { name: 'Ajustes' })).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('lang', 'es');
  await page.getByRole('combobox', { name: 'Idioma' }).selectOption('pt-BR');
  await page.getByRole('radio', { name: 'Claro' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.keyboard.press('Escape');
  await page.getByTestId('tab-test').focus();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByTestId('tab-calibration')).toHaveAttribute('aria-selected', 'true');
});
