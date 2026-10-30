import { describe, expect, it } from 'vitest';
import { updateErrorReason } from './updater';

describe('updateErrorReason', () => {
  it('treats a missing latest.json as "no release yet"', () => {
    expect(updateErrorReason('Could not fetch a valid release JSON from the remote')).toBe(
      'noRelease',
    );
    expect(updateErrorReason('server returned 404 Not Found')).toBe('noRelease');
  });

  it('recognises network failures', () => {
    expect(updateErrorReason('error sending request for url (https://github.com/…)')).toBe(
      'offline',
    );
    expect(updateErrorReason('dns error: failed to lookup address')).toBe('offline');
  });

  it('keeps anything else as a raw failure', () => {
    expect(updateErrorReason('signature verification failed')).toBe('other');
  });
});
