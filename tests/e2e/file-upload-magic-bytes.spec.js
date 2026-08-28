// @ts-check
const { test, expect } = require('@playwright/test');

// A file that claims to be a WASM binary (`contract.wasm`, `application/wasm`)
// but whose bytes are actually a DOS/PE executable — it starts with `MZ`
// (0x4D 0x5A). This is the classic "rename malware.exe to contract.wasm"
// bypass that the extension/MIME check alone cannot catch and that the
// hardened `useFileUpload` hook must now reject client-side, before any bytes
// leave the browser (the backend `upload_sanitization/magic.rs` guard is the
// server-side counterpart).
const SPOOFED_WASM = Buffer.from([0x4d, 0x5a, 0x90, 0x00, 0x03, 0x00, 0x00, 0x00]);

// `\0asm` + version 1 — the minimal valid WebAssembly module header.
const GENUINE_WASM = Buffer.from([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);

const WASM_ERROR = 'This file does not appear to be a valid WASM binary';

// Requests that would represent the file actually being uploaded/scanned. A
// correct client-side guard rejects the spoofed file before any of these fire.
function isUploadRequest(req) {
  return req.method() !== 'GET' && /upload|scan|analyze|contract/i.test(req.url());
}

test.describe('FileUploadZone – client-side magic byte validation (#432)', () => {
  test('rejects an .exe renamed to .wasm without issuing an upload request', async ({ page }) => {
    // Capture every request the page makes so we can prove the rejected file
    // never triggers a network upload.
    /** @type {import('@playwright/test').Request[]} */
    const requests = [];
    page.on('request', req => requests.push(req));

    await page.goto('/');

    // The scanner opens on the "Paste Code" tab; switch to file upload.
    await page.getByRole('button', { name: /Upload Files/i }).click();

    const uploadRequestsBefore = requests.filter(isUploadRequest).length;

    // Feed the spoofed binary to the (visually hidden) file input.
    await page.locator('input[type="file"]').setInputFiles({
      name: 'contract.wasm',
      mimeType: 'application/wasm',
      buffer: SPOOFED_WASM,
    });

    // The specific, content-derived error message must be shown to the user...
    await expect(page.getByText(WASM_ERROR)).toBeVisible();
    // ...the row must be marked Failed, never Ready...
    await expect(page.getByText('Failed')).toBeVisible();
    await expect(page.getByText('Ready', { exact: true })).toHaveCount(0);

    // ...and no upload/scan request may have been issued for the rejected file.
    const uploadRequestsAfter = requests.filter(isUploadRequest).length;
    expect(uploadRequestsAfter).toBe(uploadRequestsBefore);
  });

  test('accepts a correctly-signed .wasm binary (control)', async ({ page }) => {
    await page.goto('/');
    await page.getByRole('button', { name: /Upload Files/i }).click();

    await page.locator('input[type="file"]').setInputFiles({
      name: 'contract.wasm',
      mimeType: 'application/wasm',
      buffer: GENUINE_WASM,
    });

    // A genuine WASM header must not trigger the magic-byte rejection.
    await expect(page.getByText(WASM_ERROR)).toHaveCount(0);
  });
});
