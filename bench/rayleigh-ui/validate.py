"""Full fixed-coverage comparison of the UI engine with the retained offline model."""
from export import R,O,P,s
from concurrent.futures import ThreadPoolExecutor
import subprocess,json,sys,numpy as np
from scipy.io import wavfile
BIN=R/'target/release/examples/rayleigh_live'
GEOMS=['close','player_actual']
def original(geom,stage,n,v):
 d=s.energy.O/'validation'/stage/'recovered' if geom=='close' else s.O/'validation'/stage/'player_actual/baseline'
 return d/f'n{n}v{v}.wav'
def render(job):
 geom,stage=job;notes=list(range(93,109)) if stage=='attack' else [97,99,102,108];vs=[45,56,100] if stage=='attack' else list(range(28,128));d=O/'validation'/geom/stage;d.mkdir(parents=True,exist_ok=True)
 cmd=[BIN,O/geom/'bundle.toml',d,','.join(map(str,notes)),','.join(map(str,vs)),1.1 if stage=='attack' else .2,.1 if stage=='attack' else .05,512,1]
 if '--reuse' not in sys.argv:
  with open(d/'render.log','w') as log:subprocess.run(list(map(str,cmd)),stdout=log,stderr=subprocess.STDOUT,check=True)
 print('rendered',geom,stage,flush=True)
 return geom,stage,notes,vs
results={};models={g:{} for g in GEOMS};dense={g:{} for g in GEOMS}
with ThreadPoolExecutor(2) as ex:
 for geom,stage,notes,vs in ex.map(render,[(g,st) for g in GEOMS for st in ['attack','dense']]):
  rows=[]
  for n in notes:
   for v in vs:
    p=O/'validation'/geom/stage/f'n{n}v{v}.wav';sr,x=wavfile.read(p);sr0,y=wavfile.read(original(geom,stage,n,v));assert sr==sr0==48000 and x.shape==y.shape and x.shape[1]==2 and np.isfinite(x).all()
    err=x.astype(float)-y;db=10*np.log10(np.mean(err**2)/(np.mean(y.astype(float)**2)+1e-30)+1e-30)
    assert db < -140,(geom,stage,n,v,db)
    rows.append(dict(note=n,velocity=v,error_db=db,peak=float(abs(x).max())))
    (models if stage=='attack' else dense)[geom][f'{n}:{v}']=s.prev.measure(p,n)
  results[f'{geom}/{stage}']=dict(files=len(rows),worst_error_db=max(x['error_db'] for x in rows),peak=max(x['peak'] for x in rows),rows=rows)
old=json.load(open(R/'bench/rayleigh-c8-player/validation-summary.json'))
base=json.load(open(R/'bench/main-revalidation/models.json'))['base']
refs=json.load(open(R/'bench/main-revalidation/references.json'))
rows=s.c.fixed_rows({'live':models['close'],'base':base},refs)
ids=lambda rr:{tuple(r[k] for k in ['source','note','velocity','k']) for r in rr}
assert ids(rows)==ids(old['attack_rows']) and len(rows)==459
sus={}
for n in [96,97,99,102,105,108]:
 par=s.PAR[str(n)];k=np.arange(1,5);freq=par['f0']*k*np.sqrt(1+par['B']*k*k)
 for v in [45,56,100]:sus[f'{n}:{v}']=s.c.attack(s.c.b.load(str(O/'validation/close/attack'/f'n{n}v{v}.wav'),stereo=True),freq,.5,.5)
srows=[]
for r in old['sustain_rows']:
 x={k:r[k] for k in ['source','note','velocity','k','target']};key=f"{r['note']}:{r['velocity']}";i=r['k']-2
 x['values']={'live':sus[key]['partials'][i],'previous':r['values']['baseline']};srows.append(x)
assert ids(srows)==ids(old['sustain_rows']) and len(srows)==71
attackdelta=max(abs(a['values']['live']-b['values']['baseline']) for a in rows for b in old['attack_rows'] if ids([a])==ids([b]))
sustain_delta=max(abs(r['values']['live']-r['values']['previous']) for r in srows)
dips={}
previous=json.load(open(R/'bench/rayleigh-c8-player/validation-dense.json'))
for geom in GEOMS:
 dips[geom]={}
 for n in [97,99,102,108]:
  out=[]
  for k in range(3):
   y=[dense[geom][f'{n}:{v}']['partials'][k] for v in range(28,128)]
   z=[previous[geom]['baseline'][f'{n}:{v}']['partials'][k] for v in range(28,128)]
   if any(x is None for x in y+z):out.append(None);continue
   a=float(max(np.maximum.accumulate(y)-y));b=float(max(np.maximum.accumulate(z)-z));assert abs(a-b)<.001
   out.append(dict(live=a,previous=b,delta=a-b))
  dips[geom][n]=out
summary=dict(latency_compensation_samples=128,attack_rows=459,sustain_rows=71,maximum_attack_change_db=attackdelta,maximum_sustain_change_db=sustain_delta,files=results,dips=dips)
s.c.save(P/'validation.json',summary);s.c.save(P/'attack-rows.json',rows);s.c.save(P/'sustain-rows.json',srows)
print(json.dumps(s.c.clean({k:v for k,v in summary.items() if k not in ['files','dips']}),indent=2),flush=True)

assert attackdelta<.001 and sustain_delta<.05
