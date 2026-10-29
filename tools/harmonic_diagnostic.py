#!/usr/bin/env python3
"""Stereo timbre diagnostic: separate overtone shape from fundamental decay.

python3 tools/harmonic_diagnostic.py --run out/bench/harmonic_local --keys 60 61 62 63 64 66 67 68 69

No nasality score is fitted. Reference recordings must be available locally.
"""
import argparse
import json
import os

import numpy as np

import bench as b

WINDOWS = [(0, .06), (.06, .14), (.2, .2), (.4, .2), (.6, .2), (.8, .2), (1, .2), (.5, .5)]


def describe(x, midi):
    levels = np.array([b.stereo_partial_levels(x, midi, start, duration) for start, duration in WINDOWS])
    # A fixed set of overtones, excluding H1, anchors the shape. Missing
    # members invalidate the normalisation rather than silently changing it.
    shape = np.full_like(levels, np.nan)
    for i, row in enumerate(levels):
        if np.isfinite(row[1:6]).all():
            norm = 10 * np.log10(np.sum(10 ** (row[1:6] / 10)))
            shape[i] = row - norm
    # One fixed per-recording level for every time window; unlike Hn/H1,
    # a weakening fundamental cannot artificially lift all later harmonics.
    attack = levels[0][np.isfinite(levels[0])]
    decay = levels - np.max(attack) if len(attack) else np.full_like(levels, np.nan)
    # Each partial also has its own fixed attack anchor: isolates decay
    # rate from initial spectral balance. Unreadable attack => no curve.
    decay_self = levels - levels[0]
    return dict(shape=shape, decay=decay, decay_self=decay_self, levels=levels)


def pack(values):
    return [[round(float(x), 3) if np.isfinite(x) else None for x in row] for row in values]


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--run', default='out/bench/harmonic_local')
    ap.add_argument('--prototype-run', help='optional experimental render run (notes/ or harmonic_notes/)')
    ap.add_argument('--keys', nargs='+', type=int, default=[60, 61, 62, 63, 64, 66, 67, 68, 69])
    args = ap.parse_args()
    with open(b.REFS) as f: sources = json.load(f)['sources']
    output = dict(spec=dict(channels='mean spectral powers; RMS onset', windows=WINDOWS,
                            shape_anchor='sum of peak powers H2-H6; all five required',
                            decay_anchor='strongest H1-H8 peak in first 60 ms',
                            decay_self_anchor='same partial peak in first 60 ms'), notes={}, models={})
    for label, directory in [('model', args.run), ('prototype', args.prototype_run)]:
        if directory is None: continue
        metadata = os.path.join(directory, 'result.json')
        with open(metadata) as f: provenance = json.load(f)
        output['models'][label] = dict(commit=provenance.get('commit'), run=directory,
                                      label=provenance.get('label'), bundle=provenance.get('bundle'))
    for midi in args.keys:
        for layer, velocity in b.LAYERS.items():
            paths = dict(model=next((p for d in ['notes', 'harmonic_notes']
                                     for p in [b.find(os.path.join(args.run, d), midi, layer)] if p), None))
            if paths['model'] is None:
                raise FileNotFoundError(f'model missing {b.name(midi)} layer {layer}')
            if args.prototype_run:
                paths['prototype'] = next((p for d in ['notes', 'harmonic_notes']
                                      for p in [b.find(os.path.join(args.prototype_run, d), midi, layer)] if p), None)
                if paths['prototype'] is None:
                    raise FileNotFoundError(f'prototype comparison missing {b.name(midi)} layer {layer}')
            paths.update({nm: b.find(os.path.join(b.ROOT, s['dir']), midi, layer)
                          for nm, s in sources.items() if s['role'] == 'target'})
            measurements = {}
            for nm, path in paths.items():
                if path:
                    measurements[nm] = {k: pack(v) for k, v in describe(b.load_stereo(path), midi).items()}
            output['notes'][f'{midi}|{velocity}'] = measurements
    path = os.path.join(args.run, 'harmonic_diagnostic.json')
    with open(path, 'w') as f: json.dump(output, f, indent=1, allow_nan=False)
    print(path)


if __name__ == '__main__':
    main()
