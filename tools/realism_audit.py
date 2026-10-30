#!/usr/bin/env python3
"""Audit the existing score and stereo/mono measurement sensitivity.

No model fitting or reference mutation. Run from any directory; bench.py is
imported beside this script. See docs/realism_audit.md for interpretation.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import subprocess

import numpy as np
import bench


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def score_audit(metrics, refs):
    rows = {}
    for name, source in refs['sources'].items():
        if source['role'] != 'target':
            continue
        others = {'sources': {n: s for n, s in refs['sources'].items() if n != name}}
        rows[name] = bench.score(source['metrics'], others)
    model = bench.score(metrics, refs)
    common = set(model).intersection(*(set(z) for z in rows.values()))
    def summarize(z):
        result = bench.summary(z)
        result['n'] = len(z)
        result['equal_family_rms'] = float(np.sqrt(np.mean(np.square(list(result['families'].values())))))
        return result
    return {
        'common_keys': sorted(common),
        'model': summarize(model),
        'model_common': summarize({k: model[k] for k in sorted(common)}),
        'real_leave_one_out': {
            name: {'available': summarize(z), 'common': summarize({k: z[k] for k in sorted(common)}),
                   'model_same_targets_common': summarize({k: v for k, v in bench.score(
                       metrics, {'sources': {n: s for n, s in refs['sources'].items() if n != name}}
                   ).items() if k in common})}
            for name, z in rows.items()
        },
        'worst_model_measures': sorted((dict(key=k, **v) for k, v in model.items()),
                                       key=lambda row: abs(row['z']), reverse=True)[:15],
    }


def stereo_power(x):
    return np.mean(np.square(x), axis=1)


def cancellation_db(x):
    """Energy of (L+R)/2 relative to mean channel energy: 0 for L=R.

    This is a downmix diagnostic, not an error or a realism score.
    """
    return float(10 * np.log10((np.mean(np.square(x.mean(axis=1))) + 1e-30) /
                              (np.mean(stereo_power(x)) + 1e-30)))


def read_stereo(path):
    raw = subprocess.run(['sox', str(path), '-t', 'f32', '-r', str(bench.SR),
                          '-c', '2', '-'], check=True, capture_output=True).stdout
    return np.frombuffer(raw, dtype=np.float32).astype(float).reshape(-1, 2)


def audio_metrics(path, key):
    x = read_stereo(path)
    if len(x) < bench.SR:
        raise ValueError(f'{path}: need at least one second')
    # A shared onset preserves interchannel delays. Pressure threshold is
    # consistent with bench.onset, but uses both channels without cancellation.
    a = np.sqrt(stereo_power(x))
    if a.max() == 0:
        raise ValueError(f'{path}: silent audio')
    onset = int(np.argmax(a > 0.1 * a.max()))
    out = {'file': str(path), 'midi': key, 'peak_dbfs': float(20*np.log10(np.abs(x).max()))}
    for label, start, end in [('attack', 0, .06), ('sustain', .5, 1.)]:
        seg = x[onset + int(start*bench.SR):onset + int(end*bench.SR)]
        if len(seg) != int(end*bench.SR)-int(start*bench.SR):
            raise ValueError(f'{path}: missing {label} window')
        # Same Hann and loose partial search as bench.partials; no SNR gate:
        # p2 is diagnostic only and is not used as a new calibration target.
        size = max(len(seg), 4096)
        spectrum = np.fft.rfft(seg*np.hanning(len(seg))[:, None], size, axis=0)
        power = np.abs(spectrum)**2
        mono = np.abs(spectrum.mean(axis=1))**2
        freqs = np.fft.rfftfreq(size, 1/bench.SR)
        def partial(p, k):
            fk = k*bench.f0(key)*np.sqrt(1+4e-4*k*k*(1 if key>60 else .3))
            mask = (freqs > fk*.94) & (freqs < fk*1.08)
            return float(p[mask].max())
        stereo = power.mean(axis=1)
        out[label] = {
            'mono_loss_db': cancellation_db(seg),
            'p2_over_p1_stereo_db': float(10*np.log10((partial(stereo,2)+1e-30)/(partial(stereo,1)+1e-30))),
            'p2_over_p1_mono_db': float(10*np.log10((partial(mono,2)+1e-30)/(partial(mono,1)+1e-30))),
        }
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--model-result', type=Path, default=Path(bench.BASELINE))
    ap.add_argument('--out', type=Path, required=True)
    ap.add_argument('--renderer', type=Path, help='Also render C4–C7 at five velocities in three configurations')
    ap.add_argument('--reference', action='append', default=[], metavar='NAME=NOTESET')
    a = ap.parse_args()
    root = Path(bench.ROOT)
    a.out.mkdir(parents=True, exist_ok=True)
    model = json.loads(a.model_result.read_text())
    result = {'model_result': str(a.model_result), 'model_sha256': digest(a.model_result),
              'references_sha256': digest(bench.REFS),
              'scores': score_audit(model['metrics'], json.loads(Path(bench.REFS).read_text()))}
    if a.renderer:
        result['renderer_sha256'] = digest(a.renderer)
        result['preset_sha256'] = digest(root/'presets/default.toml')
        configurations = {'default': [], 'close_live_touch': bench.listening_args(),
                          'close_playback': bench.listening_args()+bench.playback_args()}
        result['configurations'] = configurations
        tasks = [(cfg, key, vel) for cfg in configurations for key in (60,72,84,96)
                 for vel in (30,45,56,70,100)]
        def render(task):
            cfg, key, vel = task
            path = a.out/f'{cfg}-{key}-{vel}.wav'
            cmd = [str(a.renderer.resolve()), 'note', str(path.resolve()), '--note', str(key),
                   '--velocity', str(vel), '--hold', '1.2', '--tail', '.1',
                   '--preset', str(root/'presets/default.toml')] + configurations[cfg]
            run = subprocess.run(cmd, check=True, capture_output=True, text=True)
            return dict(source=cfg, velocity=vel, command=cmd, render_log=run.stdout.strip(),
                        **audio_metrics(path, key))
        with ThreadPoolExecutor(4) as pool:
            result['probes'] = list(pool.map(render, tasks))
    for item in a.reference:
        name, folder = item.split('=', 1)
        for key in (60,72,84,96):
            for layer in (4,8,13):
                path = bench.find(folder, key, layer)
                if path:
                    result.setdefault('reference_probes', []).append(dict(
                        source=name, layer=layer, **audio_metrics(path, key)))
    (a.out/'audit.json').write_text(json.dumps(result, indent=2, allow_nan=False)+'\n')
    print(json.dumps({'model': result['scores']['model'],
                      'common_metrics': len(result['scores']['common_keys']),
                      'model_common_rms': result['scores']['model_common']['score'],
                      'real_common_rms': {n:r['common']['score'] for n,r in result['scores']['real_leave_one_out'].items()},
                      'probes': len(result.get('probes',[])), 'output':str(a.out/'audit.json')}, indent=2))


if __name__ == '__main__':
    main()
