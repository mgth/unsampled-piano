"""Render the selected passive coupling on the actual Rayleigh prototype.

Compare to the currently auditioned h=0.3 bundle with identical geometry,
velocities, duration and real-reference coverage. Does not launch the app.
"""
import study as s
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import json,math,shutil,subprocess,hashlib,tomllib
source=s.O/'prototype_early_h03';bundle=s.O/'prototype_plateau';bundle.mkdir(exist_ok=True)
selected={62:(.5,.85),64:(.7,.75),69:(.5,.85)}
release_horizontal={62:.12}
for p in source.glob('*.toml'):shutil.copy2(p,bundle/p.name)
if not (bundle/'responses.f64').exists():(bundle/'responses.f64').symlink_to((source/'responses.f64').resolve())
for n,(h,fraction) in selected.items():
 fields={'polarization.h_coupling':h,'polarization.cross':math.sqrt(h)*fraction}
 if n in release_horizontal:fields['damper.horizontal']=release_horizontal[n]
 (bundle/f'n{n}.toml').write_text(s.preset(fields))
 p=tomllib.loads((bundle/f'n{n}.toml').read_text());old=tomllib.loads((source/f'n{n}.toml').read_text())
 assert p['polarization']['cross']**2<=p['polarization']['h_coupling']
 p['polarization']=old['polarization']
 if n in release_horizontal:p['damper']['horizontal']=old['damper']['horizontal']
 assert p==old
p=bundle/'bundle.toml';p.write_text(p.read_text().replace('TEST H1 début de tenue · h0.3 Ré4 Mi4 La4 · close studio','TEST H1 · gain initial et plateau réduit · close studio'))
exe=s.R/'out/bench/experimental_b3b2808/build/release/examples/rayleigh_live'
fp=subprocess.check_output([exe,'--fingerprint'],text=True).strip();assert fp==tomllib.loads(p.read_text())['physics_fingerprint']
notes=[48,60,61,62,63,64,65,66,67,68,69,81,102,108];vel=[45,56,100]

def job(name,path):
 out=s.O/name
 subprocess.run(list(map(str,[exe,path,out,','.join(map(str,notes)),','.join(map(str,vel)),4,.5,128,1])),check=True,capture_output=True)
 rows={}
 for n in notes:
  for v in vel:
   x=s.b.load_stereo(str(out/f'n{n}v{v}.wav'))
   rows[f'{n}|{v}']={k:s.hd.pack(a) for k,a in s.hd.describe(x,n,fft_padding=1).items()}
   rows[f'{n}|{v}']['late_levels']=s.hd.pack([s.b.stereo_partial_levels(x,n,t,.2,fft_padding=1) for t in [1.5,2,3]])
 print('done',name,flush=True);return name,rows
with ThreadPoolExecutor(2) as pool:
 futures=[pool.submit(job,'plateau_control',source/'bundle.toml'),pool.submit(job,'plateau_candidate',p)]
 results=dict(f.result() for f in futures)
checks={}
sha=lambda path:hashlib.sha256(path.read_bytes()).hexdigest()
for n in notes:
 for v in vel:
  f=f'n{n}v{v}.wav';a=s.O/'plateau_control'/f;b=s.O/'plateau_candidate'/f
  checks[f'{n}|{v}']=dict(previous_sha256=sha(a),candidate_sha256=sha(b),identical=a.read_bytes()==b.read_bytes())
manifest={p.name:sha(p) for p in bundle.iterdir() if p.is_file()}
(s.O/'plateau_validation.json').write_text(json.dumps(dict(selected=selected,release_horizontal=release_horizontal,notes=notes,velocities=vel,results=results,checks=checks,bundle_manifest=manifest,renderer_sha256=sha(exe)),indent=1))
