"""No re-tuning: new default, retained contacts, full radiator; frozen observations."""
from common import *
from concurrent.futures import ThreadPoolExecutor
from scipy.io import wavfile
import shutil
NAMES=['default','contacts','integral']
PAR=json.load(open(P/'note-parameters.json'))
def file(geom,name,stage,n,v):return O/'validation'/geom/name/stage/f'n{n}v{v}.wav'
def job(x):
 geom,name,stage=x;ns=NOTES if stage=='attack' else DENSE;vs=VEL if stage=='attack' else list(range(28,128));d=O/'validation'/geom/name/stage;d.mkdir(parents=True,exist_ok=True)
 duration=[1.1,.1] if stage=='attack' else [.2,.05]
 if '--reuse' not in sys.argv:
  if name=='integral':cmd=[BIN/'rayleigh_live',O/geom/'bundle.toml',d,','.join(map(str,ns)),','.join(map(str,vs)),*duration,512,1];run(cmd)
  elif name=='default':run([BIN/'contact_batch',O/geom/'base.toml',d,','.join(map(str,ns)),','.join(map(str,vs)),*duration])
  else:
   for n in ns:run([BIN/'contact_batch',O/geom/f'n{n}.toml',d,n,','.join(map(str,vs)),*duration])
 m={};peak=0.
 for n in ns:
  for v in vs:
   p=file(geom,name,stage,n,v);sr,x=wavfile.read(p);assert sr==48000 and x.ndim==2 and x.shape[1]==2 and np.isfinite(x).all();peak=max(peak,float(abs(x).max()))
   value=c.measure_model(p,PAR[str(n)]);value['bands']=s.prev.bands(x);value['level_preroll']=float(b.onset_level(np.pad(x,((1024,0),(0,0)))));m[f'{n}:{v}']=value
 save(d/'metrics.json',m);print('done',geom,name,stage,'peak',peak,flush=True);return geom,name,stage,m,peak

def scores(rows):
 names=rows[0]['values'] if rows else []
 out={}
 for name in names:
  by={src:[r['values'][name]-r['target'] for r in rows if r['source']==src] for src in {r['source'] for r in rows}}
  out[name]=dict(mae=np.mean([np.mean(np.abs(e)) for e in by.values()]),bias=np.mean([np.mean(e) for e in by.values()]),rows=len(rows),sources=len(by))
 return out

def main():
 models={g:{n:{} for n in NAMES} for g in GEOMS};dense={g:{n:{} for n in NAMES} for g in GEOMS};checks={}
 with ThreadPoolExecutor(4) as ex:
  for g,n,st,m,peak in ex.map(job,[(g,n,st) for g in GEOMS for n in NAMES for st in ['attack','dense']]):
   (models if st=='attack' else dense)[g][n]=m;checks[f'{g}/{n}/{st}']=dict(files=len(m),peak=peak)
 old=json.load(open(R/'bench/rayleigh-c8-player/validation-summary.json'));oldm=json.load(open(R/'bench/rayleigh-c8-player/validation-models.json'));oldd=json.load(open(R/'bench/rayleigh-c8-player/validation-dense.json'))
 for g in GEOMS:
  models[g]['previous_integral']=oldm[g]['baseline'];dense[g]['previous_integral']=oldd[g]['baseline']
 base=json.load(open(R/'bench/main-revalidation/models.json'))['base'];models['close']['previous_default']=base
 refs=json.load(open(R/'bench/main-revalidation/references.json'));rows=c.fixed_rows({**models['close'],'base':base},refs)
 for r in rows:r['values'].pop('base');r['snr'].pop('base')
 ids=lambda rr:{tuple(r[k] for k in ['source','note','velocity','k']) for r in rr}
 assert ids(rows)==ids(old['attack_rows']) and len(rows)==459
 sus={}
 for name in NAMES:
  sus[name]={}
  for n in [96,97,99,102,105,108]:
   p=PAR[str(n)];k=np.arange(1,5);f=p['f0']*k*np.sqrt(1+p['B']*k*k)
   for v in VEL:sus[name][f'{n}:{v}']=c.attack(b.load(str(file('close',name,'attack',n,v)),stereo=True),f,.5,.5)
 srows=[]
 for r in old['sustain_rows']:
  x={k:r[k] for k in ['source','note','velocity','k','target']};key=f"{r['note']}:{r['velocity']}";i=r['k']-2
  x['values']={name:sus[name][key]['partials'][i] for name in NAMES};x['values']['previous_integral']=r['values']['baseline'];x['snr']={name:sus[name][key]['snr'][i] for name in NAMES};srows.append(x)
 assert ids(srows)==ids(old['sustain_rows']) and len(srows)==71
 dips={};targets={}
 for g in GEOMS:
  dips[g]={};targets[g]={}
  for name,m in dense[g].items():
   dips[g][name]={}
   for n in DENSE:
    ds=[]
    for k in range(3):
     y=[m[f'{n}:{v}']['partials'][k] for v in range(28,128)];ds.append(float(max(np.maximum.accumulate(y)-y)) if all(t is not None for t in y) else None)
    y=[m[f'{n}:{v}']['level_preroll'] for v in range(28,128)] if 'level_preroll' in m[f'{n}:28'] else None
    dips[g][name][n]=dict(partials=ds,level=float(max(np.maximum.accumulate(y)-y)) if y else None)
  for n in DENSE:
   for v in VEL:
    key=f'{n}:{v}';targets[g][key]={}
    for name in NAMES:
     m=models[g][name][key]
     prevpath=(s.energy.O/'validation/attack/recovered' if g=='close' else s.O/'validation/attack/player_actual/baseline')/f'n{n}v{v}.wav';prev=s.prev.measure(prevpath,n)
     targets[g][key][name]=dict(partials=m['partials'],delta_previous=dict(level=m['level_preroll']-prev['level_preroll'],band_1_2k=m['bands']['1-2k']-prev['bands']['1-2k']))
 subsets={'top':[r for r in rows if r['note']>=96],'transition':[r for r in rows if r['note']<96],'fsharp':[r for r in rows if r['note']==102],'c8':[r for r in rows if r['note']==108],'sustain':srows}
 for n in DENSE:
  for v in VEL:subsets[f'n{n}v{v}']=[r for r in rows if r['note']==n and r['velocity']==v]
 result=dict(scores={key:scores(rr) for key,rr in subsets.items()},dips=dips,targets=targets,files=checks,attack_rows=rows,sustain_rows=srows,retuned=False,coverage=dict(attack=459,sustain=71,attack_files_per_setup_and_profile=48,dense_files_per_setup_and_profile=400))
 save(P/'models.json',models);save(P/'dense.json',dense);save(P/'summary.json',result)
 print(json.dumps(c.clean(result['scores']),indent=1),flush=True)
if __name__=='__main__':main()
