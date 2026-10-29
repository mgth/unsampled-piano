"""Render the six retained MIDI performances with a prepared concert H1 bundle.

Requires concert-midi/renderer/target/release/h1-midi-render and bundle/bundle.toml.
Keeps float WAV masters; players use MP3 with one shared attenuation, if needed.
"""
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import hashlib, json, subprocess, tomllib
import numpy as np
from scipy.io import wavfile

R = Path(__file__).resolve().parents[2]
O = R / 'out/bench/fundamental-sustain/concert-midi'
EXE = O / 'renderer/target/release/h1-midi-render'
BUNDLE = O / 'bundle/bundle.toml'

def sha(p):
    with p.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()

def render(midi):
    wav = O / (midi.stem + '.wav')
    cmd = list(map(str, [EXE, BUNDLE, midi, wav, 5]))
    p = subprocess.run(cmd, capture_output=True, text=True)
    (O / (midi.stem + '.log')).write_text(p.stdout + p.stderr)
    p.check_returncode()
    sr, x = wavfile.read(wav)
    assert sr == 48000 and x.ndim == 2 and x.shape[1] == 2 and np.isfinite(x).all()
    peak = float(abs(x).max())
    assert peak > 0
    print(p.stdout.strip(), flush=True)
    return dict(midi=str(midi), midi_sha256=sha(midi), wav=str(wav),
                wav_sha256=sha(wav), duration_s=len(x)/sr, peak=peak, command=cmd)

if __name__ == '__main__':
    mids = sorted((R / 'data/midi').glob('*.mid'))
    assert len(mids) == 6
    fingerprint = subprocess.check_output([EXE, '--fingerprint'], text=True).strip()
    assert fingerprint == tomllib.loads(BUNDLE.read_text())['physics_fingerprint']
    with ThreadPoolExecutor(3) as pool:
        rows = list(pool.map(render, mids))
    # No per-piece loudness normalization. Reserve 2 dB for MP3 intersample peaks.
    gain_db = min(0., -2 - 20*np.log10(max(r['peak'] for r in rows)))
    for row in rows:
        mp3 = Path(row['wav']).with_suffix('.mp3')
        subprocess.run(['ffmpeg', '-nostdin', '-v', 'error', '-y', '-i', row['wav'],
                        '-af', f'volume={gain_db}dB', '-codec:a', 'libmp3lame',
                        '-b:a', '256k', str(mp3)], check=True)
        row.update(mp3=str(mp3), mp3_sha256=sha(mp3))
        print('Player ready:', mp3.name, flush=True)
    manifest = dict(physics_fingerprint=fingerprint, renderer_sha256=sha(EXE),
                    source_candidate='keyboard/adaptive/prototype_adaptive',
                    listening=['Microphones/concert.toml', 'Room/concert_hall.toml'],
                    bundle_files={p.name:sha(p) for p in BUNDLE.parent.iterdir()},
                    spatial_step_m=.005, delay_oversampling=32, sample_rate=48000,
                    block=128, threads=1, tail_s=5, common_player_gain_db=float(gain_db),
                    performances=rows)
    (O / 'manifest.json').write_text(json.dumps(manifest, indent=2, allow_nan=False)+'\n')
