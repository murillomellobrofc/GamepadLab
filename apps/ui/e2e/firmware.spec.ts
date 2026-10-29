import { expect, test } from '@playwright/test';

test('firmware calibration on the demo pad: temporary, then permanent', async ({ page }) => {
  await page.goto('/');
  await page.getByTestId('empty-demo').click();
  await page.getByTestId('tab-calibration').click();
  await page.getByRole('radio', { name: 'On the controller (firmware)' }).click();

  await expect(page.getByText('Simulated on the demo controller')).toBeVisible();
  await expect(page.getByTestId('fw-status')).toContainText('DualSense');
  await expect(page.getByTestId('fw-status')).toContainText('Temporary (safe)');
  // Nothing to save before a calibration.
  await expect(page.getByTestId('fw-save')).toBeDisabled();

  // Centre, four corners.
  await page.getByTestId('fw-center-start').click();
  for (let i = 0; i < 4; i++) {
    await expect(page.getByTestId('fw-center')).toContainText(`${i + 1}/4`);
    await page.getByTestId('fw-center-next').click();
  }
  await expect(page.getByTestId('fw-status')).toContainText('Temporary calibration active');

  // Range: both sticks rotate in the demo, so it unlocks once covered.
  await page.getByTestId('fw-range-start').click();
  await expect(page.getByTestId('fw-range-finish')).toBeEnabled({ timeout: 20_000 });
  await page.getByTestId('fw-range-finish').click();

  // Permanent save needs the checkbox and a confirmation.
  await expect(page.getByTestId('fw-save')).toBeDisabled();
  await page.getByTestId('fw-ack').check();
  await page.getByTestId('fw-save').click();
  await page.getByTestId('confirm-ok').click();
  await expect(page.getByText('Calibration saved to the controller.')).toBeVisible();
  await expect(page.getByTestId('fw-status')).toContainText('No pending changes');

  await page.getByTestId('tab-history').click();
  await expect(page.getByTestId('history-table')).toContainText('Firmware');
});
