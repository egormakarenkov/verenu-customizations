import { afterEach, describe, expect, it, vi } from 'vitest';
import { encodePcmWav, loadSessionFixture } from './devSession';

describe('browser session response failures', () => {
  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
    vi.unstubAllGlobals();
    vi.unstubAllEnvs();
    vi.resetModules();
  });

  async function connectedSession() {
    vi.useFakeTimers();
    vi.stubEnv('VITE_VERENU_SESSION', '1');
    vi.stubGlobal('window', { addEventListener: vi.fn() });
    vi.stubGlobal('location', { hash: '' });
    vi.stubGlobal('sessionStorage', { getItem: () => 'public-test-token' });
    const fetcher = vi.fn(async () => new Response(JSON.stringify({
      transport: 'rust-live', id: import.meta.env.VITE_VERENU_SESSION_ID,
      eventCursor: 0, cursor: 0, events: [],
    }), { status: 200 }));
    vi.stubGlobal('fetch', fetcher);
    vi.resetModules();
    const session = await import('./devSession');
    await session.initializeDevSession();
    return { session, fetcher };
  }

  it('rejects an interrupted successful response instead of returning null to model stores', async () => {
    const { session, fetcher } = await connectedSession();
    const interrupted = new TypeError('Failed to fetch');
    fetcher.mockResolvedValueOnce({ ok: true, json: () => Promise.reject(interrupted) } as Response);
    await expect(session.sessionRequest('/invoke')).rejects.toBe(interrupted);
  });

  it('keeps a useful error when an unsuccessful response has no readable JSON body', async () => {
    const { session, fetcher } = await connectedSession();
    fetcher.mockResolvedValueOnce({ ok: false, json: () => Promise.reject(new SyntaxError('Not JSON')) } as Response);
    await expect(session.sessionRequest('/invoke')).rejects.toThrow('Dev backend is unavailable');
  });
});

describe('browser PCM conversion', () => {
  it('rejects fixture loading outside an authenticated browser session', async () => {
    await expect(loadSessionFixture('synthetic.wav')).rejects.toThrow('No browser dev session');
  });
  it('writes mono PCM16 WAV headers and clamps capture samples', () => {
    const bytes = encodePcmWav(new Float32Array([-2, 0, 2]));
    const view = new DataView(bytes.buffer);
    expect(new TextDecoder().decode(bytes.subarray(0, 4))).toBe('RIFF');
    expect(view.getUint32(24, true)).toBe(16000);
    expect(view.getUint16(22, true)).toBe(1);
    expect(view.getUint32(40, true)).toBe(6);
    expect(view.getInt16(44, true)).toBe(-32768);
    expect(view.getInt16(48, true)).toBe(32767);
  });
  it('rejects nonfinite samples, wrong rates, and overlong recordings', () => {
    expect(() => encodePcmWav(new Float32Array([NaN]))).toThrow();
    expect(() => encodePcmWav(new Float32Array(1), 48000)).toThrow();
    expect(() => encodePcmWav(new Float32Array(120 * 16000 + 1))).toThrow();
  });
});
