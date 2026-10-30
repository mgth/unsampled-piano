"""Frozen contact profiles crossed with rectangular and grand-outline soundboards.
No re-fitting or production default changes. Run stages: render, score, full, check.
"""
import sys,json,re,tomllib,hashlib,subprocess,os
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import numpy as np
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools'))
import contact_voicing as c
b=c.b
OUT=ROOT/'out/real-geometry';OUT.mkdir(parents=True,exist_ok=True)
BIN=Path(os.environ.get('PIANO_GEOMETRY_BIN', '/tmp/piano-real-geometry-target/release'));c.BATCH=BIN/'examples/contact_batch';c.PROBE=BIN/'examples/top_probe'
BASE=c.BASE_PRESET.read_text();OLD=ROOT/'bench/contact-residuals';PARAMS=json.load(open(ROOT/'bench/contact-voicing/note-parameters.json'));REFS=json.load(open(ROOT/'bench/contact-voicing/references.json'))
GUARD=json.load(open(ROOT/'bench/contact-voicing/profiles.json'))['guarded'];LOCAL=json.load(open(OLD/'profile-local.json'))['notes']
OUTLINE=[[0,0],[.12,0],[.30,.10],[.50,.21],[.70,.28],[.85,.30],[.95,.33],[1,.40],[1,1],[0,1]]
NAMES=[f'{g}_{v}' for g in ['rect','outline','calibrated'] for v in ['base','guarded','local']]
def geometry(text,g):
 if g=='rect':return text
 block=re.search(r'(?ms)^\[soundboard\]\n.*?(?=^\[|\Z)',text).group()
 new=block+'outline = '+json.dumps(OUTLINE)+'\noutline_below = 1500.0\n\n'
 if g=='calibrated':
  for key,value in dict(rho_h=8.4,dx=4590.,dy=2040.,impedance_slope=.0024,impedance_slope_bass=.0076).items():new=re.sub(rf'^{key} = .*$',f'{key} = {value}',new,flags=re.M)
 return text.replace(block,new)
def spec(v,n):
 if v=='base':return c.DEFAULT,{}
 if v=='guarded':return c.interpolate(GUARD,n),{}
 e=LOCAL[str(n)];return e['params'],e['extra']
def contact_fields(s,extra):
 return dict(hardness=float(f"{.3*s['q']*.2**(-s['p']):.12g}"),p_offset=float(f"{s['p']:.12g}"),hysteresis=float(f"{s['h']:.12g}"),contact_width_scale=float(f"{s['w']:.12g}"),height_spread=float(f"{s['d']:.12g}"),strike_ratio=extra.get('strike_ratio',.12))
def prepare():
 for name in NAMES:
  g,v=name.split('_');text=geometry(BASE,g)
  if v!='base':
   for n in range(93,109):
    text+='\n[[treble_contact]]\n'
    for key,value in contact_fields(*spec(v,n)).items():text+=f'{key} = {value}\n'
  p=OUT/'presets'/f'{name}.toml';p.parent.mkdir(parents=True,exist_ok=True);p.write_text(text)
 c.save(OUT/'design.json',dict(geometries=['rect','outline','calibrated'],voicings=['base','guarded','local'],refit=False,notes=list(range(93,109)),velocities=[45,56,100],dense_notes=[97,99,102,108],dense_velocities=list(range(28,128)),outline=OUTLINE,outline_below=1500,table_calibration=dict(rho_h=8.4,dx=4590,dy=2040,impedance_slope=.0024,impedance_slope_bass=.0076)))
def render():
 prepare()
 def one(name):
  preset=OUT/'presets'/f'{name}.toml';d=OUT/'attack'/name
  c.run([c.BATCH,preset,d,','.join(map(str,range(93,109))),'45,56,100',1.1,.1])
  a={f'{n}:{v}':c.measure_model(d/f'n{n}v{v}.wav',PARAMS[str(n)]) for n in range(93,109) for v in [45,56,100]};c.save(d/'metrics.json',a);print(name,'attack complete',flush=True)
  d=OUT/'dense'/name;c.run([c.BATCH,preset,d,'97,99,102,108',','.join(map(str,range(28,128))),.2,.05])
  c.save(d/'metrics.json',{f'{n}:{v}':c.measure_model(d/f'n{n}v{v}.wav',PARAMS[str(n)]) for n in [97,99,102,108] for v in range(28,128)});print(name,'100 velocities complete',flush=True)
 with ThreadPoolExecutor(4) as pool:list(pool.map(one,NAMES))
def summaries(rows,names):
 result={}
 for split,pred in dict(all_top=lambda r:r['note']>=96,targets=lambda r:r['note'] in [97,99,102],held_sources=lambda r:r['note']>=96 and r['source'] in c.HOLDOUT,pp=lambda r:r['note']>=96 and r['velocity']==45,transition=lambda r:r['note']<96).items():
  sub=[r for r in rows if pred(r)];result[split]={}
  for name in names:
   errs={s:[r['values'][name]-r['target'] for r in sub if r['source']==s] for s in REFS};errs={s:e for s,e in errs.items() if e}
   result[split][name]=dict(mae=np.mean([np.mean(np.abs(e)) for e in errs.values()]),bias=np.mean([np.mean(e) for e in errs.values()]),rows=len(sub),sources=len(errs),low_snr=sum(r['snr'][name]<12 for r in sub))
 return result

def score():
 models={name:json.load(open(OUT/'attack'/name/'metrics.json')) for name in NAMES}
 # The exact same original baseline fixes physical eligibility for every geometry.
 models['base']=json.load(open(ROOT/'bench/contact-voicing/validation-models.json'))['base']
 rows=c.fixed_rows(models,REFS);scores=summaries(rows,NAMES)
 # Freeze the previous sustained observations; never reselect on a new candidate.
 sus=json.load(open(OLD/'sustain-check.json'));srows=[];sm={}
 for name in NAMES:
  sm[name]={}
  for n in [96,97,99,102,105,108]:
   p=PARAMS[str(n)];k=np.arange(1,5);f=p['f0']*k*np.sqrt(1+p['B']*k*k)
   for v in [45,56,100]:sm[name][f'{n}:{v}']=c.attack(b.load(str(OUT/'attack'/name/f'n{n}v{v}.wav'),stereo=True),f,.5,.5)
 for old in sus['coverage_rows']:
  r={k:old[k] for k in ['source','note','velocity','k','target']};key=f"{r['note']}:{r['velocity']}";i=r['k']-2
  r['values']={name:sm[name][key]['partials'][i] for name in NAMES};r['snr']={name:sm[name][key]['snr'][i] for name in NAMES}
  assert all(x is not None for x in r['values'].values());srows.append(r)
 dips={};dense={}
 for name in NAMES:
  dense[name]=json.load(open(OUT/'dense'/name/'metrics.json'));a=dense[name];dips[name]={}
  for n in [97,99,102,108]:
   vals=[[a[f'{n}:{v}']['partials'][i] for v in range(28,128)] for i in range(3)];lv=[a[f'{n}:{v}']['level'] for v in range(28,128)]
   dips[name][n]=dict(partials=[dict(drop=float(max(np.maximum.accumulate(y)-y)),minimum=min(y)) if all(x is not None for x in y) else None for y in vals],level_dip=max(np.maximum.accumulate(lv)-lv))
 targets=[]
 for n,v,k in [(102,100,3),(99,45,3),(97,45,2),(108,45,2),(108,56,2),(108,100,2)]:
  key=f'{n}:{v}';r=[a[key]['partials'][k-2] for a in REFS.values() if key in a and a[key]['partials'][k-2] is not None]
  targets.append(dict(note=n,velocity=v,k=k,target_mean=np.mean(r),target_sd=np.std(r),reference_count=len(r),values={name:a[key]['partials'][k-2] for name,a in models.items() if name!='base'},levels={name:a[key]['level'] for name,a in models.items() if name!='base'}))
 c.save(OUT/'summary.json',dict(reference_protocol=b.ANALYSIS_VERSION,analysis_version='contact-physical-peaks-v1',attack_scores=scores,sustain_scores=summaries(srows,NAMES),targets=targets,dips=dips,attack_rows=rows,sustain_rows=srows));c.save(OUT/'models.json',models);c.save(OUT/'dense-models.json',dense)
 print(json.dumps(c.clean(dict(scores=scores['all_top'],targets=targets)),indent=1))

def full():
 # Every geometry/voicing comparison keeps the same 90 strikes and durations.
 names=['outline_base','outline_local','calibrated_base','calibrated_local','calibrated_guarded']
 def one(name):
  d=OUT/'full'/name;notes=d/'notes';p=OUT/'presets'/f'{name}.toml'
  c.run([c.BATCH,p,notes,','.join(map(str,b.KEYS)),'45,56,100',20,.5])
  for n in b.KEYS:
   for l,v in b.LAYERS.items():(notes/f'n{n}v{v}.wav').replace(notes/f'{b.name(n)}v{l}.wav')
  b.survey(str(notes),str(d/'survey.csv'));metrics=b.measure_set(str(notes),str(d/'survey.csv'));refs=json.load(open(ROOT/'bench/references.json'));z=b.score(metrics,refs);base=json.load(open(ROOT/'bench/protocol-v2/result.json'))
  c.save(d/'result.json',dict(analysis_version=b.ANALYSIS_VERSION,profile=name,metrics=metrics,z=z,summary=b.summary(z)));b.report(name,z,{},base,str(d/'scorecard.md'));print(name,b.summary(z),flush=True)
 with ThreadPoolExecutor(3) as pool:list(pool.map(one,names))
if __name__=='__main__':globals()[sys.argv[1]]()
