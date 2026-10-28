"""Fixed-coverage validation of two local F#7 candidates.
The protected control is the earlier local profile, with D#7 returned to main.
Every other key, including the preserved C8 voicing, is held identical.
No production per-key implementation or default is changed.
"""
from pathlib import Path
import sys,json,hashlib,shutil
from concurrent.futures import ThreadPoolExecutor
import numpy as np
R=Path(__file__).resolve().parents[2];sys.path.insert(0,str(R/'tools'));sys.path.insert(0,str(Path(__file__).parent))
import contact_voicing as c
from diagnose import patch
b=c.b;P=R/'bench/fsharp-contact';O=R/'out/fsharp-contact/validation';O.mkdir(parents=True,exist_ok=True)
BASE=(R/'bench/main-revalidation/effective.toml').read_text();LOCAL=json.load(open(R/'bench/contact-residuals/profile-local.json'))['notes']
PAR=json.load(open(R/'bench/main-revalidation/note-parameters.json'));REF=json.load(open(R/'bench/main-revalidation/references.json'))
screen={x['name']:x for x in json.load(open(P/'damped-law-results.json'))}
PROFILES={'protected':LOCAL['102'],'damped':screen['e026']['params'],'balanced':screen['e035']['params']}
NAMES=list(PROFILES);VEL=[45,56,100];NOTES=list(range(93,109));DENSE=[97,99,102,108]
BIN=R/'out/main-revalidation/bin';BATCH=BIN/'contact_batch';SURVEY=BIN/'piano-survey'
def preset(name,n):
 p=O/'presets'/name/f'n{n}.toml'
 entry={'params':c.DEFAULT,'extra':{}} if n==99 else LOCAL[str(n)]
 if n==102:entry=PROFILES[name]
 c.write_preset(p,BASE,**entry['params']);p.write_text(patch(p.read_text(),entry['extra']));return p

def render():
 c.save(P/'selected.json',dict(profiles=PROFILES,selection='Screened on the same references; no blind holdout. e026 minimizes attack MAE among smooth candidates; e035 better preserves pp. Both are experimental.',protected_notes=[97,99,108],base_policy='old local profile except note 99 restored to main',dense_velocities=list(range(28,128))))
 def one(job):
  name,n=job;p=preset(name,n);d=O/'attack'/name
  c.run([BATCH,p,d,n,','.join(map(str,VEL)),1.1,.1])
  m={f'{n}:{v}':c.measure_model(d/f'n{n}v{v}.wav',PAR[str(n)]) for v in VEL}
  if n in DENSE:
   d=O/'dense'/name;c.run([BATCH,p,d,n,','.join(map(str,range(28,128))),.2,.05])
   c.save(d/f'n{n}.json',{f'{n}:{v}':c.measure_model(d/f'n{n}v{v}.wav',PAR[str(n)]) for v in range(28,128)})
  print('attack',name,n,flush=True);return name,m
 models={n:{} for n in NAMES}
 with ThreadPoolExecutor(6) as ex:
  for name,m in ex.map(one,[(name,n) for name in NAMES for n in NOTES]):models[name].update(m)
 c.save(P/'validation-models.json',models)
 def long(job):
  name,n=job;d=O/'full'/name/'notes';p=preset(name,n)
  c.run([BATCH,p,d,n,','.join(map(str,VEL)),20,.5])
  for l,v in b.LAYERS.items():(d/f'n{n}v{v}.wav').replace(d/f'{b.name(n)}v{l}.wav')
  print('full',name,n,flush=True)
 with ThreadPoolExecutor(6) as ex:list(ex.map(long,[(name,n) for name in NAMES for n in b.KEYS]))
 for name in NAMES:
  d=O/'full'/name;c.run([SURVEY,d/'notes','--out',d/'survey.csv','--channel-power'])
  metrics=b.measure_set(str(d/'notes'),str(d/'survey.csv'));z=b.score(metrics,json.load(open(R/'bench/references.json')))
  c.save(P/f'full-{name}.json',dict(metrics=metrics,z=z,summary=b.summary(z)));print(name,b.summary(z),flush=True)

def scores(rows,names):
 result={}
 for name in names:
  errors={s:[r['values'][name]-r['target'] for r in rows if r['source']==s] for s in {r['source'] for r in rows}}
  result[name]=dict(mae=np.mean([np.mean(np.abs(e)) for e in errors.values()]),bias=np.mean([np.mean(e) for e in errors.values()]),rows=len(rows),sources=len(errors),low_snr=sum(r['snr'][name]<12 for r in rows))
 return result

def score():
 models=json.load(open(P/'validation-models.json'));models['base']=json.load(open(R/'bench/main-revalidation/models.json'))['base']
 rows=c.fixed_rows(models,REF);prev=json.load(open(R/'bench/main-revalidation/summary.json'))
 ident=lambda rows:{(r['source'],r['note'],r['velocity'],r['k']) for r in rows}
 assert ident(rows)==ident(prev['attack_rows'])
 sus={}
 for name in NAMES:
  sus[name]={}
  for n in [96,97,99,102,105,108]:
   p=PAR[str(n)];k=np.arange(1,5);freq=p['f0']*k*np.sqrt(1+p['B']*k*k)
   for v in VEL:sus[name][f'{n}:{v}']=c.attack(b.load(str(O/'attack'/name/f'n{n}v{v}.wav'),stereo=True),freq,.5,.5)
 srows=[]
 for old in prev['sustain_rows']:
  r={k:old[k] for k in ['source','note','velocity','k','target']};key=f"{r['note']}:{r['velocity']}";i=r['k']-2
  r['values']={n:sus[n][key]['partials'][i] for n in NAMES};r['snr']={n:sus[n][key]['snr'][i] for n in NAMES}
  r['values']['base']=old['values']['base'];r['snr']['base']=old['snr']['base'];assert all(v is not None for v in r['values'].values());srows.append(r)
 dense={};dips={};levels={}
 for name in NAMES:
  dense[name]={};dips[name]={};levels[name]={}
  for n in DENSE:
   m=json.load(open(O/'dense'/name/f'n{n}.json'));dense[name].update(m);dips[name][n]=[]
   for i in range(3):
    a=[m[f'{n}:{v}']['partials'][i] for v in range(28,128)]
    dips[name][n].append(float(max(np.maximum.accumulate(a)-a)) if all(x is not None for x in a) else None)
   lv=[m[f'{n}:{v}']['level'] for v in range(28,128)];levels[name][n]=float(max(np.maximum.accumulate(lv)-lv))
 sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
 # Rendered outputs, not just equal preset values, protect the other keys.
 equal={}
 for stage in ['attack','dense','full']:
  root=O/stage/'protected';files=list(root.rglob('*.wav'));kept=[]
  for p in files:
   if (stage=='full' and b.key_of(p.name)==102) or (stage!='full' and p.name.startswith('n102v')):continue
   kept.append(p)
  assert all(sha(p)==sha(O/stage/name/p.relative_to(root)) for p in kept for name in NAMES[1:]);equal[stage]=len(kept)
 # Protected notes against previous measured controls.
 for n,control in [(97,'base'),(99,'base'),(108,'local')]:
  for v in VEL:assert sha(O/'attack/protected'/f'n{n}v{v}.wav')==sha(R/f'out/main-revalidation/attack/{control}'/f'n{n}v{v}.wav')
 z={name:json.load(open(P/f'full-{name}.json'))['z'] for name in NAMES};common=set.intersection(*(set(a) for a in z.values()))
 summary=dict(attack=scores([r for r in rows if r['note']>=96],list(models)),fsharp=scores([r for r in rows if r['note']==102],list(models)),sustain=scores(srows,list(models)),fsharp_sustain=scores([r for r in srows if r['note']==102],list(models)),dips=dips,level_dips=levels,unchanged_rendered_files=equal,full={n:dict(native_components=len(z[n]),common_components=len(common),summary=b.summary({k:z[n][k] for k in common})) for n in NAMES},attack_rows=rows,sustain_rows=srows)
 c.save(P/'validation-summary.json',summary);c.save(P/'validation-dense.json',dense)
 print(json.dumps(c.clean({k:v for k,v in summary.items() if k not in ['attack_rows','sustain_rows']}),indent=1),flush=True)
if __name__=='__main__':render();score()
