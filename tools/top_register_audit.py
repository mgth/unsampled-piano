#!/usr/bin/env python3
"""Reproduce the top-register causal audit without changing production settings.

Build `cargo build --release -p piano-render --example top_probe` first.
Run `python3 tools/top_register_audit.py screen --out out/top-register`, then
`... compare --out out/top-register` (90 notes per candidate, corrected bench).
--measure-only remeasures existing renders, never substitutes missing files.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import io
import json
from pathlib import Path
import re
import subprocess

import numpy as np
import bench as b

ROOT = Path(b.ROOT)
KEYS = [96, 99, 102, 105, 108]
WINDOWS = [(0, .06), (.05, .4), (.6, .4)]
# Overrides are applied after close/studio/playback; each is an isolated experiment.
OVERLAYS = {
    'base': '', 'no_action': '[noise]\naction=0\n', 'no_keybed': '[noise]\nkeybed=0\n',
    'no_frame': '[frame]\nenabled=false\n', 'no_rim': '[rim]\nenabled=false\n',
    'no_sympathy': '[sympathetic]\nenabled=false\n', 'dry': '[room]\nenabled=false\n', 'overkill': '',
    'soft15': 'hardness_slope_top=0.15\n', 'soft10': 'hardness_slope_top=0.10\n',
    'action50': '[noise]\naction=0.05\n', 'action_long': '[noise]\naction_time=0.5\n',
    'sympathy2': '[sympathetic]\ngain=2.0\n', 'rigid': 'shank_share=0.0\n',
    'listen_all': '', 'soft10_listen_all': 'hardness_slope_top=0.10\n',
}
PROBES = ['base', 'listen_all', 'thresholds', 'contact2', 'uncapped', 'nonlinear', 'force']
CANDIDATES = ['soft10', 'soft15', 'action50', 'action_long', 'sympathy2', 'listen_all', 'soft10_listen_all']
ABLATIONS = ['base', 'no_action', 'no_keybed', 'no_frame', 'no_rim', 'no_sympathy', 'dry', 'overkill']
SCREEN = ABLATIONS + ['soft15', 'soft10', 'action50', 'action_long', 'sympathy2', 'rigid']
EXE = ROOT / 'target/release/unsampled-piano-render'
PROBE = ROOT / 'target/release/examples/top_probe'


def clean(x):
    if isinstance(x, dict): return {str(k): clean(v) for k, v in x.items()}
    if isinstance(x, (list, tuple, np.ndarray)): return [clean(v) for v in x]
    if isinstance(x, (float, np.floating)): return float(x) if np.isfinite(x) else None
    return x


def save(path, value):
    path.write_text(json.dumps(clean(value), indent=1, allow_nan=False) + '\n')


def run(cmd):
    return subprocess.run(list(map(str, cmd)), check=True, capture_output=True, text=True)


def prepare(out):
    out.mkdir(parents=True, exist_ok=True)
    run([EXE, 'dump-preset', out / 'effective.toml', *b.listening_args(), *b.playback_args()])
    for name, text in OVERLAYS.items():
        (out / f'{name}.toml').write_text(text)
        run([EXE, 'dump-preset', out / f'{name}-effective.toml', '--preset', out / 'effective.toml',
             '--listening', out / f'{name}.toml'])
    paths = ['presets/default.toml', 'bench/references.json', 'tools/bench.py',
             'crates/piano-render/examples/top_probe.rs', 'tools/top_register_audit.py']
    paths += [str(EXE), str(PROBE)]
    save(out / 'manifest.json', dict(analysis_version=b.ANALYSIS_VERSION, commit=b.git_commit(),
         hashes={p: hashlib.sha256((ROOT / p).read_bytes()).hexdigest() for p in paths},
         keys=KEYS, layers=b.LAYERS, windows=WINDOWS, overrides=OVERLAYS))


def render(out, variant, m, layer, full=False):
    d = out / 'full' / variant / 'notes' if full else out / variant
    d.mkdir(parents=True, exist_ok=True)
    wav = d / f'{b.name(m)}v{layer}.wav'
    hold, tail = (20, .5) if full else (1.3, .1)
    if variant.startswith('probe_') or variant.endswith('listen_all') or full:
        mode = variant.removeprefix('probe_') if variant.startswith('probe_') else ('listen_all' if variant.endswith('listen_all') else 'base')
        preset = out / ('effective.toml' if variant.startswith('probe_') else f'{variant}-effective.toml')
        run([PROBE, preset, m, b.LAYERS[layer], mode, wav, hold, tail])
    else:
        cmd = [EXE, 'note', wav, '--note', m, '--velocity', b.LAYERS[layer], '--hold', hold, '--tail', tail,
               '--preset', out / f'{variant}-effective.toml']
        if variant == 'overkill': cmd += ['--quality', 'overkill']
        run(cmd)
    print(f'{variant} {b.name(m)} layer {layer}', flush=True)


def energy_bands(x, start, t, duration, centers=(1000, 2000, 4000, 8000)):
    """Channel-mean, Hann-weighted mean-square energy, comparable across lengths.

    No waveform downmix. Same start for model ablations; recording onset per take.
    """
    seg = x[start + round(t*b.SR): start + round((t+duration)*b.SR)]
    if len(seg) != round(duration*b.SR): raise ValueError('Incomplete analysis window')
    p = b.spectrum_power(seg) * 2 / (len(seg) * np.sum(np.hanning(len(seg))**2))
    f = np.fft.rfftfreq(len(seg), 1/b.SR)
    return np.array([10*np.log10(p[(f >= c/np.sqrt(2)) & (f < c*np.sqrt(2))].sum()+1e-30) for c in centers])


def measure(x, m, start=None):
    start = b.onset(x) if start is None else start
    return dict(onset_ms=1000*start/b.SR, measured_onset_ms=1000*b.onset(x)/b.SR,
        partials=b.partials(x, m, 0, .06, [2, 3, 4]),
        bands={f'{t:g}-{t+d:g}': energy_bands(x, start, t, d) for t, d in WINDOWS},
        bench_att=b.band_levels(x, .05, [1000, 2000], .4) - b.band_levels(x, 0, [31.5,63,125,250,500,1000,2000,4000,8000], .1).max())


def screen_measure(out):
    models = {v: {} for v in SCREEN + ['probe_'+p for p in PROBES]}
    for m in KEYS:
        for layer in (8, 13):
            key = f'{m}:{layer}'
            base = b.load(str(out / 'base' / f'{b.name(m)}v{layer}.wav'), stereo=True)
            for v, rows in models.items():
                x = b.load(str(out / v / f'{b.name(m)}v{layer}.wav'), stereo=True)
                rows[key] = measure(x, m, b.onset(base))
                if v in ABLATIONS and v != 'base': rows[key]['removed'] = measure(base-x, m, b.onset(base))
    refs = {}
    sources = json.loads((ROOT / 'bench/references.json').read_text())['sources']
    for name, src in sources.items():
        if src['role'] != 'target': continue
        refs[name] = {}
        for m in KEYS:
            for layer in (8, 13):
                path = b.find(str(ROOT / src['dir']), m, layer)
                if path: refs[name][f'{m}:{layer}'] = measure(b.load(path, stereo=True), m)
    save(out / 'diagnostics.json', dict(analysis_version=b.ANALYSIS_VERSION,
        variants={v: OVERLAYS[v] for v in ABLATIONS}, model={v: models[v] for v in ABLATIONS}, references=refs))
    save(out / 'probes.json', {p: models['probe_'+p] for p in PROBES})
    save(out / 'candidates.json', {v: models[v] for v in SCREEN if v not in ABLATIONS})
    # Equally weight pianos, pair identical notes and valid components; retain counts.
    paired = {}
    for field, layer, labels in [('partials', 8, ['p2','p3','p4']), ('bench_att', 13, ['1k','2k'])]:
        for i, label in enumerate(labels):
            pairs = {}
            for name, rows in refs.items():
                ks = [k for k,r in rows.items() if k.endswith(f':{layer}') and np.isfinite(r[field][i]) and np.isfinite(models['base'][k][field][i])]
                if ks:
                    pairs[name] = dict(n=len(ks), keys=ks, reference=np.mean([rows[k][field][i] for k in ks]),
                                      model=np.mean([models['base'][k][field][i] for k in ks]))
            paired[field+'|'+label] = pairs
    save(out / 'paired.json', paired)
    curves(out, sources)
    contacts(out)


def curves(out, sources):
    times = np.arange(0, 1.01, .05)
    def curve(x):
        start = b.onset(x)
        ref = energy_bands(x, start, 0, .1, [31.5,63,125,250,500,1000,2000,4000,8000]).max()
        return np.array([energy_bands(x, start, t, .1, [1000,2000])-ref for t in times])
    result = dict(times=times, duration=.1, references={}, model={})
    for name, src in sources.items():
        if src['role'] != 'target': continue
        paths = [b.find(str(ROOT/src['dir']), m, 13) for m in KEYS]
        result['references'][name] = np.mean([curve(b.load(p, stereo=True)) for p in paths if p], axis=0)
    for v in ['base','soft10','action50','action_long','sympathy2','probe_listen_all']:
        result['model'][v] = np.mean([curve(b.load(str(out/v/f'{b.name(m)}v13.wav'), stereo=True)) for m in KEYS], axis=0)
    save(out / 'curves.json', result)


def contacts(out):
    rows = []
    for v in ['base','soft15','soft10','rigid']:
        for m in KEYS:
            for layer in (8,13):
                result = run([PROBE, out/f'{v}-effective.toml', m, b.LAYERS[layer], 'contact', 'unused'])
                meta = {k: float(v) for k,v in re.findall(r'(\w+)=([-\d.eE+]+)', result.stderr)}
                force = np.loadtxt(io.StringIO(result.stdout), delimiter=',', skiprows=1)[:,1:]
                t = np.arange(len(force))/b.SR
                k = np.arange(1,5); f = meta['f0']*k*np.sqrt(1+meta['B']*k*k)
                # Sum string energies, not forces: different contact phases cannot cancel.
                e = np.array([np.sum(np.abs(np.exp(-2j*np.pi*fr*t) @ force)**2) for fr in f])
                rows.append(dict(variant=v, layer=layer, **meta, force_harmonics_db=10*np.log10(e/e[0])))
    save(out / 'contact.json', rows)


def compare(out):
    refs = json.loads((ROOT/'bench/references.json').read_text())
    if any(s.get('analysis_version') != b.ANALYSIS_VERSION for s in refs['sources'].values()):
        raise ValueError('Rebuild references with the corrected bench')
    baseline = json.loads((ROOT/'bench/protocol-v2/result.json').read_text())
    for v in CANDIDATES:
        d = out/'full'/v
        b.survey(str(d/'notes'), str(d/'survey.csv'))
        metrics = b.measure_set(str(d/'notes'), str(d/'survey.csv')); z = b.score(metrics, refs)
        result = dict(analysis_version=b.ANALYSIS_VERSION, variant=v, metrics=metrics, z=z, summary=b.summary(z))
        save(d/'result.json', result)
        b.report(v, z, {}, baseline, str(d/'scorecard.md'))
        print(v, result['summary'], flush=True)
    paired_candidates(out)


def paired_candidates(out):
    """Hold note/partial support fixed across candidates and each reference.

    Reject physical partials above 12 kHz using the model and reference tracker;
    report paired dB gaps separately from the aggregate v2 scores. Requiring a
    tracked partial is a sensitivity check, not proof that an untracked attack
    partial is absent.
    """
    import pandas as pd
    variants = ['base', *CANDIDATES]
    model = {v: {} for v in variants}
    for v in variants:
        directory = ROOT/'out/bench/protocol-v2/notes' if v == 'base' else out/'full'/v/'notes'
        for m in KEYS:
            for layer in (8, 13):
                x = b.load(b.find(str(directory), m, layer), stereo=True)
                model[v][f'{m}:{layer}'] = b.partials(x, m, 0, .06, [2, 3, 4])
    refs = json.loads((out/'diagnostics.json').read_text())['references']
    contact = json.loads((out/'contact.json').read_text())
    params = {int(r['note']): r for r in contact if r['variant'] == 'base' and r['layer'] == 8}
    surveys = {name: pd.read_csv(ROOT/'out/bench/refs'/f'{name}.survey.csv') for name in refs}
    rows = {}
    for layer in (8, 13):
        for i, k in enumerate((2, 3, 4)):
            sources = {}
            for name, reference_rows in refs.items():
                valid = []
                for m in KEYS:
                    key = f'{m}:{layer}'
                    p, survey = params[m], surveys[name]
                    frequencies = survey[(survey.midi == m) & (survey.layer == layer) & (survey.k == k)].freq
                    physical_frequency = p['f0']*k*np.sqrt(1+p['B']*k*k)
                    if physical_frequency >= 12000 or not len(frequencies) or frequencies.max() >= 12000:
                        continue
                    if key not in reference_rows or reference_rows[key]['partials'][i] is None:
                        continue
                    if any(not np.isfinite(model[v][key][i]) for v in variants):
                        continue
                    valid.append(key)
                if valid:
                    reference = np.mean([reference_rows[key]['partials'][i] for key in valid])
                    means = {v: np.mean([model[v][key][i] for key in valid]) for v in variants}
                    sources[name] = dict(keys=valid, reference=reference, model=means)
            rows[f'{layer}:p{k}'] = dict(sources=sources, mean_gap_db={
                v: np.mean([s['model'][v]-s['reference'] for s in sources.values()]) for v in variants})
    save(out/'paired-candidates.json', rows)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('stage', choices=['screen','compare'])
    ap.add_argument('--out', type=Path, default=ROOT/'out/top-register')
    ap.add_argument('--measure-only', action='store_true')
    args = ap.parse_args(); out = args.out.resolve()
    if not args.measure_only:
        prepare(out)
        variants = SCREEN + ['probe_'+p for p in PROBES] if args.stage == 'screen' else CANDIDATES
        keys = KEYS if args.stage == 'screen' else b.KEYS
        layers = (8,13) if args.stage == 'screen' else b.LAYERS
        jobs = [(out,v,m,l,args.stage=='compare') for v in variants for m in keys for l in layers]
        with ThreadPoolExecutor(6) as ex: list(ex.map(lambda j: render(*j), jobs))
    (screen_measure if args.stage=='screen' else compare)(out)


if __name__ == '__main__': main()
