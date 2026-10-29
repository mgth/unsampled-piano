"""Prepare concert MIDI rendering using an explicitly supplied experimental checkout.

Usage: python3 bench/fundamental-sustain/prepare_concert_midi.py EXPERIMENTAL_ROOT
Requires its frozen out/upstream-4a00f00/bin tools and Rayleigh filter generator,
and the local keyboard/adaptive/prototype_adaptive candidate from this study.
"""
from pathlib import Path
import argparse, importlib.util, json, struct, subprocess, tomllib
import numpy as np

R = Path(__file__).resolve().parents[2]
O = R / 'out/bench/fundamental-sustain/concert-midi'
SOURCE = R / 'out/bench/fundamental-sustain/keyboard/adaptive/prototype_adaptive'

def run(args):
    subprocess.run(list(map(str, args)), check=True)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('experimental_root', type=Path)
    args = parser.parse_args()
    root = args.experimental_root.resolve()
    bins = root / 'out/upstream-4a00f00/bin'
    bundle = tomllib.loads((SOURCE / 'bundle.toml').read_text())
    fingerprint = subprocess.check_output([bins / 'rayleigh_live', '--fingerprint'], text=True).strip()
    if fingerprint != bundle['physics_fingerprint']:
        raise ValueError('Candidate and frozen tools have different physics fingerprints')
    dest = O / 'bundle'
    dest.mkdir(parents=True, exist_ok=True)
    listening = [R / 'presets/listening' / p for p in
                 ['Microphones/concert.toml', 'Room/concert_hall.toml']]
    for p in SOURCE.glob('*.toml'):
        if p.name == 'bundle.toml':
            continue
        run([bins / 'unsampled-piano-render', 'dump-preset', dest / p.name,
             '--preset', p, '--listening', listening[0], '--listening', listening[1]])
        before, after = tomllib.loads(p.read_text()), tomllib.loads((dest / p.name).read_text())
        assert {k:v for k,v in before.items() if k not in ['room','soundboard']} == {
            k:v for k,v in after.items() if k not in ['room','soundboard']}
    (dest / 'bundle.toml').write_text((SOURCE / 'bundle.toml').read_text().replace(
        'close studio', 'concert microphones · concert_hall'))
    run([bins / 'rayleigh_kernel', dest / 'base.toml', O / 'kernel', .005, 32])
    spec = importlib.util.spec_from_file_location('concert_filters', root / 'bench/rayleigh-audio/filters.py')
    filters = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(filters)
    filters.P = O
    filters.make(O / 'kernel')
    ir = np.load(O / 'kernel/responses.npz')['driven']
    sb = tomllib.loads((dest / 'base.toml').read_text())['soundboard']
    assert ir.shape == (16, 2, 65536)
    (dest / 'responses.f64').write_bytes(b'RAYL002\n' + struct.pack('<4I', 48000, 16, 2, 65536)
        + (ir * 10**(sb['mic_gain_db']/20)).astype('<f8').tobytes())
    project = O / 'renderer'
    project.mkdir(exist_ok=True)
    # JSON string quoting is compatible with TOML basic strings, including paths.
    quote = lambda p: json.dumps(str(p))
    (project / 'Cargo.toml').write_text(f'''[workspace]
[package]
name = "h1-midi-render"
version = "0.1.0"
edition = "2024"
[[bin]]
name = "h1-midi-render"
path = {quote(Path(__file__).with_name('midi_render.rs'))}
[dependencies]
piano-core = {{ path = {quote(root / 'crates/piano-core')}, features = ["experimental-rayleigh"] }}
piano-host = {{ path = {quote(root / 'crates/piano-host')}, features = ["experimental-rayleigh"] }}
hound = "3.5"
[profile.release]
codegen-units = 1
lto = "thin"
''')
    run(['cargo', 'build', '--offline', '--release', '--manifest-path', project / 'Cargo.toml'])
    actual = subprocess.check_output([project / 'target/release/h1-midi-render', '--fingerprint'], text=True).strip()
    if actual != fingerprint:
        raise ValueError('Experimental sources changed since the frozen tools were built')

if __name__ == '__main__':
    main()
