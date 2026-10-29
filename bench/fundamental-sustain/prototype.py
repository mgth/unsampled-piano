import study as s
from pathlib import Path
import json,subprocess,hashlib,shutil,re,sys
from concurrent.futures import ThreadPoolExecutor
E=Path(sys.argv[1]).resolve()
source=E/'out/upstream-4a00f00/close';d=s.O/'prototype_local';d.mkdir(exist_ok=True)
for p in source.glob('*.toml'):shutil.copy2(p,d/p.name)
if not (d/'responses.f64').exists():(d/'responses.f64').symlink_to(source/'responses.f64')
fields={'polarization.h_coupling':.1,'polarization.cross':.316}
# A per-key model uses the same acoustic geometry. Keep every original
# high-register contact and the source bundle's physics fingerprint.
text=(source/'bundle.toml').read_text().replace('label = ', 'label = ',1)
text=re.sub(r'^label = .*$', 'label = "TEST tenue H1 · couplage horizontal local · close studio"',text,flags=re.M)
for n in [62,63,64,66,69]:
 (d/f'n{n}.toml').write_text(s.preset(fields))
 text+=f'\n[[contacts]]\nnote = {n}\npreset = "n{n}.toml"\n'
(d/'bundle.toml').write_text(text)
exe=Path(sys.argv[2]).resolve()
notes=list(range(60,70))+[48,81,102,108];vel=[45,56,100]
def job(name,bundle):
 out=s.O/name;cmd=[exe,bundle,out,','.join(map(str,notes)),','.join(map(str,vel)),4,.5,128,1]
 subprocess.run(list(map(str,cmd)),check=True,capture_output=True)
 rows={}
 for n in notes:
  for v in vel:
   x=s.b.load_stereo(str(out/f'n{n}v{v}.wav'))
   rows[f'{n}|{v}']={k:s.hd.pack(a) for k,a in s.hd.describe(x,n,fft_padding=1).items()}
   rows[f'{n}|{v}']['late_levels']=s.hd.pack([s.b.stereo_partial_levels(x,n,t,.2,fft_padding=1) for t in [1.5,2.,3.]])
 (out/'metrics.json').write_text(json.dumps(rows,indent=1));return name,rows
with ThreadPoolExecutor(2) as pool:
 futures=[pool.submit(job,'prototype_base',source/'bundle.toml'),pool.submit(job,'prototype_test',d/'bundle.toml')]
 results=dict(f.result() for f in futures)
checks={}
for n in notes:
 for v in vel:
  a=s.O/'prototype_base'/f'n{n}v{v}.wav';b=s.O/'prototype_test'/a.name
  checks[f'{n}|{v}']=dict(base_sha=hashlib.sha256(a.read_bytes()).hexdigest(),test_sha=hashlib.sha256(b.read_bytes()).hexdigest(),identical=a.read_bytes()==b.read_bytes())
(s.O/'prototype.json').write_text(json.dumps(dict(notes=notes,velocities=vel,changed_keys=[62,63,64,66,69],fields=fields,results=results,checks=checks),indent=1))
print('prototype validated; identical controls:',sum(c['identical'] for k,c in checks.items() if int(k.split('|')[0]) not in [62,63,64,66,69]),flush=True)
