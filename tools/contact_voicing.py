#!/usr/bin/env python3
"""Experimental contact voicing: reproducible isolated strikes, no production edits.

Build piano-render's contact_batch and top_probe examples first. Profiles specify
q (felt-force factor at 0.2 mm), p (exponent increment), h (hysteresis multiplier),
d (unison height spread in metres), and w (contact width multiplier).

Examples (run screen references before scoring):
  tools/contact_voicing.py references --out out/contact-voicing
  tools/contact_voicing.py grid --grid bench/contact-voicing/grid.json --out out/contact-voicing
  tools/contact_voicing.py profile --profiles bench/contact-voicing/profiles.json --names base,guarded --out out/contact-voicing --fine
  tools/contact_voicing.py score --names base,guarded --out out/contact-voicing
  tools/contact_voicing.py bench --profiles bench/contact-voicing/profiles.json --names guarded --out out/contact-voicing
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

import numpy as np
import pandas as pd
import bench as b

ROOT = Path(b.ROOT)
ANCHORS = [96, 99, 102, 105, 108]
NOTES = list(range(93, 109))
DENSE = [32,40,45,48,52,56,60,64,68,72,80,88,96,100,108,116,127]
FINE = sorted(set(range(28, 128, 2)) | {45,127})
HOLDOUT = ['iowa', 'headroom']
BATCH = ROOT/'target/release/examples/contact_batch'
PROBE = ROOT/'target/release/examples/top_probe'
BASE_PRESET = ROOT/'out/top-register/effective.toml'
DEFAULT = dict(q=1, h=.6, p=0, w=1, d=.0001)


def clean(x):
    if isinstance(x, dict): return {str(k): clean(v) for k, v in x.items()}
    if isinstance(x, (list, tuple, np.ndarray)): return [clean(v) for v in x]
    if isinstance(x, (float, np.floating)): return float(x) if np.isfinite(x) else None
    if isinstance(x, np.integer): return int(x)
    if isinstance(x, np.bool_): return bool(x)
    return x


def save(path, x):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(clean(x), indent=1, allow_nan=False)+'\n')


def run(cmd):
    return subprocess.run(list(map(str, cmd)), check=True, capture_output=True, text=True)


def write_preset(path, base, q=1, h=.6, p=0, w=1, d=.0001):
    """Change the exponent while preserving the specified force at 0.2 mm."""
    params = tomllib.loads(base)
    replacements = dict(hardness=params['hardness']*q*.2**(-p),
        hysteresis=h, p_offset=params['p_offset']+p, contact_width_scale=w, height_spread=d)
    for key, value in replacements.items():
        base, count = re.subn(rf'^{key} = .*$', f'{key} = {value:.12g}', base, flags=re.M)
        if count != 1: raise ValueError(f'Expected exactly one {key} in complete preset')
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(base)


def interpolate(profile, note):
    points = sorted(map(int, profile))
    result = {}
    for key in DEFAULT:
        values = [profile[str(n)][key] for n in points]
        result[key] = float(np.exp(np.interp(note, points, np.log(values)))) if key == 'q' else float(np.interp(note, points, values))
    return result


def note_parameters(base_path, notes):
    result = {}
    for note in notes:
        r = run([PROBE, base_path, note, 56, 'contact', 'unused'])
        result[note] = {k: float(v) for k, v in re.findall(r'(\w+)=([-\d.eE+]+)', r.stderr)}
    return result


def attack(x, frequencies, t0=0, duration=.06):
    """Stereo-power peaks about physical/tracked frequencies, 60 ms Hann by default.

    Candidate peaks are never discarded for low SNR: support cannot improve by
    hiding a partial. Low-SNR values are upper bounds, explicitly counted later.
    Only reference eligibility is frozen at SNR >=12 dB before comparisons.
    """
    start = b.onset(x) + round(t0*b.SR)
    size = round(duration*b.SR)
    segment = x[start:start+size]
    if len(segment) != size: raise ValueError('Incomplete partial window')
    fft_size = max(16384, size)
    power = b.spectrum_power(segment, fft_size)
    f = np.fft.rfftfreq(fft_size, 1/b.SR)
    levels, snr = [], []
    for frequency in frequencies:
        if not np.isfinite(frequency) or frequency >= 12000:
            levels.append(None); snr.append(None); continue
        width = max(2*b.SR/len(segment), .015*frequency)
        peak = (abs(f-frequency) < width) & (f < 12000)
        noise = (abs(f-frequency) > 2*width) & (abs(f-frequency) < 4*width)
        if not peak.any() or not noise.any(): raise ValueError('Invalid partial window')
        level = 10*np.log10(power[peak].max()+1e-30)
        floor = 10*np.log10(np.median(power[noise])+1e-30)
        levels.append(level); snr.append(level-floor)
    fundamental = levels[0]
    return dict(partials=[None if v is None or fundamental is None else v-fundamental for v in levels[1:]],
                snr=snr[1:], p1=fundamental, level=b.onset_level(x))


def measure_model(path, params):
    k = np.arange(1, 5)
    return attack(b.load(str(path), stereo=True), params['f0']*k*np.sqrt(1+params['B']*k*k))


def references(out):
    result = {}
    sources = json.loads((ROOT/'bench/references.json').read_text())['sources']
    for name, src in sources.items():
        if src['role'] != 'target': continue
        if src.get('analysis_version') != b.ANALYSIS_VERSION: raise ValueError('Rebuild corrected references')
        rows = {}; survey = pd.read_csv(ROOT/'out/bench/refs'/f'{name}.survey.csv')
        for note in NOTES:
            for layer, velocity in b.LAYERS.items():
                path = b.find(str(ROOT/src['dir']), note, layer)
                tracked = survey[(survey.midi == note) & (survey.layer == layer)]
                if path is None or tracked.empty: continue
                first = tracked.iloc[0]; k = np.arange(1, 5)
                frequencies = first.f0*k*np.sqrt(1+first.B*k*k)
                for i, partial in enumerate(k):
                    hits = tracked[tracked.k == partial]
                    if len(hits): frequencies[i] = hits.iloc[0].freq
                row = attack(b.load(path, stereo=True), frequencies)
                for i in range(3):
                    if row['snr'][i] is None or row['snr'][i] < 12: row['partials'][i] = None
                rows[f'{note}:{velocity}'] = row
        result[name] = rows
    save(out/'references.json', result)


def render_grid(args, base):
    grid = json.loads(args.grid.read_text())
    parameters = note_parameters(args.base, sorted(set(args.notes) | {e["note"] for e in grid.values() if "note" in e}))
    save(args.out/'note-parameters.json', parameters)
    def one(item):
        name, entry = item
        spec = entry.get('params', entry)
        notes = [entry['note']] if 'note' in entry else args.notes
        preset = args.out/f'{name}.toml'; write_preset(preset, base, **spec)
        directory = args.out/args.directory/name
        run([BATCH, preset, directory, ','.join(map(str, notes)), ','.join(map(str,args.velocities)), .2, .05])
        metrics = {f'{n}:{v}': measure_model(directory/f'n{n}v{v}.wav', parameters[n]) for n in notes for v in args.velocities}
        save(directory/'metrics.json', metrics)
        print(name, flush=True)
    with ThreadPoolExecutor(6) as pool: list(pool.map(one, grid.items()))


def render_profiles(args, base, full=False):
    profiles = json.loads(args.profiles.read_text())
    notes = b.KEYS if full else NOTES
    parameters = {} if full else note_parameters(args.base, notes)
    if parameters: save(args.out/'note-parameters.json', parameters)
    def one(job):
        name, note = job
        spec = DEFAULT if name == 'base' else interpolate(profiles[name], note)
        directory = args.out/('full' if full else 'validation')/name
        preset = directory/f'n{note}.toml'; write_preset(preset, base, **spec)
        velocities = list(b.LAYERS.values()) if full else ((FINE if args.fine else DENSE) if note in ANCHORS else [45,56,72,100,116])
        run([BATCH, preset, directory/'notes', note, ','.join(map(str, velocities)), 20 if full else .2, .5 if full else .05])
        if full:
            for layer, velocity in b.LAYERS.items():
                (directory/'notes'/f'n{note}v{velocity}.wav').replace(directory/'notes'/f'{b.name(note)}v{layer}.wav')
        else:
            save(directory/f'n{note}.json', {f'{note}:{v}': measure_model(directory/'notes'/f'n{note}v{v}.wav', parameters[note]) for v in velocities})
        print(name, note, flush=True)
    with ThreadPoolExecutor(6) as pool: list(pool.map(one, [(n,m) for n in args.names for m in notes]))
    if full:
        refs = json.loads((ROOT/'bench/references.json').read_text())
        baseline = json.loads((ROOT/'bench/protocol-v2/result.json').read_text())
        for name in args.names:
            d = args.out/'full'/name
            b.survey(str(d/'notes'), str(d/'survey.csv'))
            metrics = b.measure_set(str(d/'notes'), str(d/'survey.csv')); z = b.score(metrics, refs)
            save(d/'result.json', dict(analysis_version=b.ANALYSIS_VERSION, profile=name, metrics=metrics, z=z, summary=b.summary(z)))
            b.report(name, z, {}, baseline, str(d/'scorecard.md'))


def fixed_rows(models, refs):
    """Eligibility depends only on the original baseline and the references."""
    rows = []
    for source, reference in refs.items():
        for key, ref in reference.items():
            note, velocity = map(int, key.split(':'))
            baseline = models['base'][key]
            for i, target in enumerate(ref['partials']):
                if target is None or baseline['partials'][i] is None: continue
                values = {name: model[key]['partials'][i] for name, model in models.items()}
                if any(v is None for v in values.values()): raise ValueError('Candidate lost fixed physical support')
                rows.append(dict(source=source, note=note, velocity=velocity, k=i+2, target=target, values=values,
                    snr={name: model[key]['snr'][i] for name, model in models.items()}))
    return rows


def score(args):
    models = {}
    for name in args.names:
        models[name] = {}
        for p in (args.out/'validation'/name).glob('n*.json'): models[name].update(json.loads(p.read_text()))
    refs = json.loads((args.out/'references.json').read_text()); rows = fixed_rows(models, refs)
    splits = {
        'fit_mf_ff': lambda r: r['source'] not in HOLDOUT and r['note'] in ANCHORS and r['velocity'] in [56,100],
        'held_sources_all': lambda r: r['source'] in HOLDOUT and r['note'] >=96,
        'held_notes_all': lambda r: r['note'] not in ANCHORS and r['note'] >=96,
        'pp': lambda r: r['note'] >=96 and r['velocity']==45,
        'transition': lambda r: r['note']<96,
        'all_top': lambda r: r['note']>=96,
    }
    scores = {}
    for split, eligible in splits.items():
        subset = [r for r in rows if eligible(r)]; scores[split] = {}
        for name in models:
            errors = {s: [r['values'][name]-r['target'] for r in subset if r['source']==s] for s in refs}
            errors = {s:v for s,v in errors.items() if v}
            scores[split][name] = dict(mae=np.mean([np.mean(np.abs(v)) for v in errors.values()]),
                bias=np.mean([np.mean(v) for v in errors.values()]), rms=np.sqrt(np.mean([np.mean(np.square(v)) for v in errors.values()])),
                rows=len(subset), sources=len(errors), low_snr=sum(r['snr'][name]<12 for r in subset))
    dips = {}
    for name, model in models.items():
        dips[name] = {}
        for note in ANCHORS:
            velocities = sorted(int(k.split(':')[1]) for k in model if k.startswith(f'{note}:'))
            values = [[model[f'{note}:{v}']['partials'][i] for v in velocities] for i in range(3)]
            levels = np.array([model[f'{note}:{v}']['level'] for v in velocities])
            dips[name][str(note)] = dict(velocities=velocities, partial_dips=[
                np.max(np.maximum.accumulate(y)-y) if all(v is not None for v in y) else None for y in values],
                level_dip=np.max(np.maximum.accumulate(levels)-levels))
    save(args.out/'summary.json', dict(analysis_version='contact-physical-peaks-v1', reference_protocol=b.ANALYSIS_VERSION, scores=scores, dips=dips, coverage_rows=rows))


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('stage', choices=['references','grid','profile','score','bench'])
    ap.add_argument('--out', type=Path, default=ROOT/'out/contact-voicing')
    ap.add_argument('--base', type=Path, default=BASE_PRESET)
    ap.add_argument('--grid', type=Path)
    ap.add_argument('--directory', default='grid')
    ap.add_argument('--profiles', type=Path)
    ap.add_argument('--names', type=lambda s:s.split(','), default=['base','guarded'])
    ap.add_argument('--notes', type=lambda s:list(map(int,s.split(','))), default=ANCHORS)
    ap.add_argument('--velocities', type=lambda s:list(map(int,s.split(','))), default=[56,100])
    ap.add_argument('--fine', action='store_true')
    args = ap.parse_args(); args.out = args.out.resolve(); args.out.mkdir(parents=True,exist_ok=True)
    if args.stage=='grid' and args.grid is None: ap.error('--grid required')
    if args.stage in ['profile','bench'] and args.profiles is None: ap.error('--profiles required')
    base = args.base.read_text()
    if args.stage=='references': references(args.out)
    elif args.stage=='grid': render_grid(args,base)
    elif args.stage in ['profile','bench']: render_profiles(args,base,args.stage=='bench')
    else: score(args)
    inputs = [Path(__file__),args.base,ROOT/'tools/bench.py',ROOT/'bench/references.json',BATCH,PROBE]
    inputs += [p for p in [args.grid,args.profiles] if p is not None]
    save(args.out/f'inputs-{args.stage}.json',dict(analysis_version=b.ANALYSIS_VERSION,
         stage=args.stage, notes=args.notes, velocities=args.velocities, names=args.names, fine=args.fine,
         hashes={str(p):hashlib.sha256(p.read_bytes()).hexdigest() for p in inputs}))


if __name__=='__main__': main()
