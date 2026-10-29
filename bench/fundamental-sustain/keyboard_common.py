"""Shared, reproducible helpers for the 88-key H1 investigation."""
from pathlib import Path
import hashlib,json,math,re,subprocess,tomllib
import numpy as np
import study as s
R=s.R
O=s.O/'keyboard'
O.mkdir(parents=True,exist_ok=True)
SOURCE=s.O/'prototype_plateau'
EXE=R/'out/bench/experimental_b3b2808/build/release/examples/rayleigh_live'
VELOCITIES=[45,56,100]
NOTES=list(range(21,109))
TIMES=[.03,.13,.3,.5,.7,.9,1.1]

def sha(path):
 with Path(path).open('rb') as f:return hashlib.file_digest(f,'sha256').hexdigest()

def save(path,value):
 Path(path).write_text(json.dumps(value,indent=1,allow_nan=False)+'\n')

def measure(path,n):
 x=s.b.load_stereo(str(path))
 data={k:s.hd.pack(a) for k,a in s.hd.describe(x,n,fft_padding=1).items()}
 data.update(path=str(Path(path).resolve()),sha256=sha(path),duration=len(x)/s.b.SR,peak=float(abs(x).max()))
 return data

def bundle(destination,changes):
 """Copy current auditioned contacts; modify only the requested scalar fields."""
 destination=Path(destination);destination.mkdir(exist_ok=True)
 src=tomllib.loads((SOURCE/'bundle.toml').read_text())
 contacts={c['note']:c['preset'] for c in src['contacts']}
 for p in SOURCE.glob('*.toml'):(destination/p.name).write_bytes(p.read_bytes())
 if not (destination/'responses.f64').exists():(destination/'responses.f64').symlink_to((SOURCE/'responses.f64').resolve())
 text=(SOURCE/'bundle.toml').read_text()
 text=re.sub(r'^label = .*$', 'label = "TEST H1 · recherche clavier complet · close studio"',text,flags=re.M)
 for n,fields in changes.items():
  p=SOURCE/contacts.get(n,'base.toml');before=tomllib.loads(p.read_text());edited=p.read_text()
  for key,value in fields.items():
   section,field=key.split('.')
   pattern=rf'(?ms)(^\[{re.escape(section)}\]\n(?:(?!^\[).)*?^{re.escape(field)} = )[^\n]+'
   edited,count=re.subn(pattern,lambda m:m[1]+str(float(value)),edited);assert count==1,key
  after=tomllib.loads(edited);assert after['polarization']['cross']**2<=after['polarization']['h_coupling']+1e-12
  for key in fields:
   section,field=key.split('.');before[section][field]=after[section][field]
  assert before==after
  (destination/f'n{n}.toml').write_text(edited)
  if n not in contacts:text+=f'\n[[contacts]]\nnote = {n}\npreset = "n{n}.toml"\n'
  else:assert contacts[n]==f'n{n}.toml'
 (destination/'bundle.toml').write_text(text)
 return destination/'bundle.toml'

def render(bundle_path,out,notes,velocities=VELOCITIES,hold=4,tail=.5):
 out=Path(out);out.mkdir(exist_ok=True)
 stamp=dict(bundle={p.name:sha(p) for p in Path(bundle_path).parent.iterdir() if p.is_file()},renderer=sha(EXE),notes=notes,velocities=velocities,hold=hold,tail=tail,block=128,threads=1)
 marker=out/'render.json'
 if marker.exists() and json.loads(marker.read_text())==stamp and all((out/f'n{n}v{v}.wav').exists() for n in notes for v in velocities):return
 result=subprocess.run(list(map(str,[EXE,bundle_path,out,','.join(map(str,notes)),','.join(map(str,velocities)),hold,tail,128,1])),capture_output=True,text=True)
 (out/'render.log').write_text(result.stdout+result.stderr)
 result.check_returncode();save(marker,stamp)
 print('rendered',out.name,len(notes)*len(velocities),flush=True)
