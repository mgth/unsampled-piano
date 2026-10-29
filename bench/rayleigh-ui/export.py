"""Export frozen player/close research bundles; no fitted gains or preset defaults."""
from pathlib import Path
import importlib.util,json,hashlib,struct,sys
import numpy as np
R=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('player_support',R/'bench/rayleigh-c8-player/support.py')
s=importlib.util.module_from_spec(spec);spec.loader.exec_module(s)
O=R/'out/rayleigh-ui';P=R/'bench/rayleigh-ui'
def export(geom):
 d=O/geom;d.mkdir(parents=True,exist_ok=True)
 base=s.preset(dict(params=s.c.DEFAULT,extra={}),geom,d/'base.toml')
 label='pianiste, couvercle 35°, studio' if geom=='player_actual' else 'micros rapprochés, studio'
 lines=[f'label = "Rayleigh intégral · contacts retenus · {label}"','base = "base.toml"','impulses = "responses.f64"']
 for n in range(93,109):
  e=s.CONTACT.get(str(n),s.prev.entry(n));p=s.preset(e,geom,d/f'n{n}.toml')
  lines += ['','[[contacts]]',f'note = {n}',f'preset = "n{n}.toml"']
 s.kernel(geom);ir=(s.a.IR['driven']*s.a.GAIN).astype('<f8');assert ir.shape==(16,2,65536)
 (d/'responses.f64').write_bytes(b'RAYL002\n'+struct.pack('<4I',48000,16,2,65536)+ir.tobytes())
 (d/'bundle.toml').write_text('\n'.join(lines)+'\n')
 meta=dict(geometry=geom,sample_rate=48000,taps=65536,latency=128,contacts=s.CONTACT,files={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(d.glob('*')) if p.suffix=='.toml' or p.name=='responses.f64'})
 (P/f'{geom}-bundle.json').write_text(json.dumps(meta,indent=2)+'\n')
 print(d/'bundle.toml')
if __name__=='__main__':
 raise SystemExit('Historical exporter for 63109f8 only. After upstream integration, build the new tools and use bench/upstream-4a00f00/prepare.py; old kernels are incompatible.')
