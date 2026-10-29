"""Test a stronger local H1 continuation; leave the listening bundle untouched."""
import study as s
from pathlib import Path
import json,subprocess,shutil,math,hashlib
source=s.O/'prototype_local';bundle=s.O/'prototype_early_h03';bundle.mkdir(exist_ok=True)
for p in source.glob('*.toml'):shutil.copy2(p,bundle/p.name)
if not (bundle/'responses.f64').exists():(bundle/'responses.f64').symlink_to((source/'responses.f64').resolve())
for n in [62,64,69]:
 (bundle/f'n{n}.toml').write_text(s.preset({'polarization.h_coupling':.3,'polarization.cross':math.sqrt(.3)*.999}))
p=bundle/'bundle.toml';p.write_text(p.read_text().replace('TEST tenue H1 · couplage horizontal local · close studio','TEST H1 début de tenue · h0.3 Ré4 Mi4 La4 · close studio'))
exe=s.R/'out/bench/experimental_b3b2808/build/release/examples/rayleigh_live';out=s.O/'prototype_early_test'
notes=[60,61,62,63,64,66,69];vel=[45,56,100]
subprocess.run(list(map(str,[exe,p,out,','.join(map(str,notes)),','.join(map(str,vel)),4,.5,128,1])),check=True,capture_output=True)
rows={};checks={}
for n in notes:
 for v in vel:
  f=out/f'n{n}v{v}.wav';x=s.b.load_stereo(str(f));rows[f'{n}|{v}']={k:s.hd.pack(a) for k,a in s.hd.describe(x,n).items()}
  rows[f'{n}|{v}']['late_levels']=s.hd.pack([s.b.stereo_partial_levels(x,n,t,.2) for t in [1.5,2,3]])
  old=s.O/'prototype_test'/f.name
  checks[f'{n}|{v}']=dict(sha256=hashlib.sha256(f.read_bytes()).hexdigest(),previous_sha256=hashlib.sha256(old.read_bytes()).hexdigest(),identical=f.read_bytes()==old.read_bytes())
(s.O/'early_prototype.json').write_text(json.dumps(dict(notes=notes,velocities=vel,changed_keys=[62,64,69],results=rows,checks=checks),indent=1))
print('early prototype done',flush=True)
