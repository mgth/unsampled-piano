"""Revalidate the merged engine; frozen contact candidates, unchanged defaults.
Run from this worktree: python3 bench/main-revalidation/run.py
"""
from pathlib import Path
import sys,json,hashlib,subprocess,re,tomllib,shutil
from concurrent.futures import ThreadPoolExecutor
import numpy as np
ROOT=Path(__file__).resolve().parents[2];sys.path.insert(0,str(ROOT/'tools'))
import contact_voicing as c
b=c.b
OUT=ROOT/'out/main-revalidation';RESULT=ROOT/'bench/main-revalidation'
OLD=Path('/home/user/.codex/worktrees/piano-realism-audit/piano')
NAMES=['base','guarded','local'];NOTES=list(range(93,109));VEL=[45,56,100];DENSE=[97,99,102,108]
OUT.mkdir(parents=True,exist_ok=True)
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def dump(p,x):c.save(p,x)
def prepare():
 bindir=OUT/'bin';bindir.mkdir(exist_ok=True)
 for name in ['unsampled-piano-render','piano-survey','examples/contact_batch','examples/top_probe']:
  shutil.copy2(ROOT/'target/release'/name,bindir/Path(name).name)
 c.BATCH=bindir/'contact_batch';c.PROBE=bindir/'top_probe'
 assert '--channel-power' in c.run([bindir/'piano-survey','--help']).stdout
 renderer=bindir/'unsampled-piano-render'
 c.run([renderer,'dump-preset',OUT/'compiled-default.toml'])
 c.run([renderer,'dump-preset',OUT/'effective.toml',*b.listening_args(),*b.playback_args()])
 c.run([renderer,'dump-preset',OUT/'file-default-effective.toml','--preset',ROOT/'presets/default.toml',*b.listening_args(),*b.playback_args()])
 params=c.note_parameters(OUT/'effective.toml',NOTES);dump(RESULT/'note-parameters.json',params)
 default=tomllib.loads((OUT/'compiled-default.toml').read_text());file=tomllib.loads((ROOT/'presets/default.toml').read_text())
 def diff(a,z,p=''):
  rows={}
  for k in a.keys()|z.keys():
   x=a.get(k);y=z.get(k);key=p+k
   if isinstance(x,dict) and isinstance(y,dict):rows.update(diff(x,y,key+'.'))
   elif x!=y:rows[key]={'compiled':x,'file':y}
  return rows
 dump(RESULT/'default-differences.json',diff(default,file))
 refs=json.load(open(ROOT/'bench/references.json'))
 # Reference audio and surveys are immutable inputs from the corrected bench.
 # Verify analysis implementation identity before reusing their measurements.
 analysis=['tools/bench.py','tools/plath_set.py']+[str(p.relative_to(ROOT)) for p in (ROOT/'crates/piano-analysis/src').rglob('*.rs')]
 assert all(sha(ROOT/p)==sha(OLD/p) for p in analysis)
 assert all(s['analysis_version']==b.ANALYSIS_VERSION for s in refs['sources'].values())
 plath=refs['sources']['plath'];manifest=json.load(open(Path(plath['dir'])/'manifest.json'))
 assert manifest['format_version']==2 and manifest['gain_per_pa']==.01 and len(manifest['notes'])==88
 dump(RESULT/'manifest.json',dict(commit=c.run(['git','-C',ROOT,'rev-parse','HEAD']).stdout.strip(),default_sha256=sha(ROOT/'presets/default.toml'),compiled_default_sha256=sha(OUT/'compiled-default.toml'),effective_sha256=sha(OUT/'effective.toml'),binaries={p.name:sha(p) for p in bindir.iterdir()},analysis={p:sha(ROOT/p) for p in analysis},reference_policy='Corrected reference audio/surveys reused; attack metrics remeasured. All model audio newly rendered.',reference_surveys={n:sha(ROOT/'out/bench/refs'/f'{n}.survey.csv') for n in refs['sources']},profiles_frozen=True,source_dsp_unchanged=True))
 # Regenerate reference attack values from the original calibrated stereo audio.
 c.references(RESULT)
 return params

def preset(name,n):
 base=(OUT/'effective.toml').read_text();path=OUT/'presets'/name/f'n{n}.toml'
 path.parent.mkdir(parents=True,exist_ok=True)
 if name=='base':path.write_text(base);return path
 if name=='guarded':spec=c.DEFAULT if n<93 else c.interpolate(json.load(open(ROOT/'bench/contact-voicing/profiles.json'))['guarded'],n);extra={}
 else:
  entry=json.load(open(ROOT/'bench/contact-residuals/profile-local.json'))['notes'][str(n)];spec=entry['params'];extra=entry['extra']
 c.write_preset(path,base,**spec)
 text=path.read_text()
 for k,v in extra.items():
  text,count=re.subn(rf'^{k} = .*$',f'{k} = {v}',text,flags=re.M);assert count==1
 path.write_text(text);return path

def render(params):
 def one(job):
  name,n=job;p=preset(name,n);out=OUT/'attack'/name
  c.run([c.BATCH,p,out,n,','.join(map(str,VEL)),1.1,.1])
  rows={f'{n}:{v}':c.measure_model(out/f'n{n}v{v}.wav',params[n]) for v in VEL}
  if n in DENSE:
   d=OUT/'dense'/name;c.run([c.BATCH,p,d,n,','.join(map(str,range(28,128))),.2,.05])
   dump(d/f'n{n}.json',{f'{n}:{v}':c.measure_model(d/f'n{n}v{v}.wav',params[n]) for v in range(28,128)})
  print('top',name,n,flush=True);return name,rows
 models={name:{} for name in NAMES}
 with ThreadPoolExecutor(6) as ex:
  for name,rows in ex.map(one,[(name,n) for name in NAMES for n in NOTES]):models[name].update(rows)
 dump(RESULT/'models.json',models)

def full(render_audio=True):
 def one(job):
  name,n=job;p=preset(name,n);out=OUT/'full'/name/'notes'
  c.run([c.BATCH,p,out,n,','.join(map(str,VEL)),20,.5])
  for l,v in b.LAYERS.items():(out/f'n{n}v{v}.wav').replace(out/f'{b.name(n)}v{l}.wav')
  print('full',name,n,flush=True)
 if render_audio:
  with ThreadPoolExecutor(6) as ex:list(ex.map(one,[(name,n) for name in NAMES for n in b.KEYS]))
 refs=json.load(open(ROOT/'bench/references.json'))
 for name in NAMES:
  out=OUT/'full'/name
  c.run([OUT/'bin/piano-survey',out/'notes','--out',out/'survey.csv','--channel-power'])
  metrics=b.measure_set(str(out/'notes'),str(out/'survey.csv'));z=b.score(metrics,refs)
  dump(RESULT/f'full-{name}.json',dict(analysis_version=b.ANALYSIS_VERSION,metrics=metrics,z=z,summary=b.summary(z)))
  b.report(name,z,{},None,str(RESULT/f'full-{name}.md'))
  print('score',name,b.summary(z),flush=True)

def scores(rows):
 out={}
 for name in NAMES:
  errors={s:[r['values'][name]-r['target'] for r in rows if r['source']==s] for s in {r['source'] for r in rows}}
  out[name]=dict(mae=np.mean([np.mean(np.abs(e)) for e in errors.values()]),bias=np.mean([np.mean(e) for e in errors.values()]),rows=len(rows),sources=len(errors),low_snr=sum(r['snr'][name]<12 for r in rows))
 return out

def score(params):
 models=json.load(open(RESULT/'models.json'));refs=json.load(open(RESULT/'references.json'));rows=c.fixed_rows(models,refs)
 old_refs=json.load(open(OLD/'bench/contact-voicing/references.json'));assert refs==old_refs
 oldrows=json.load(open(ROOT/'bench/contact-residuals/sustain-check.json'))['coverage_rows']
 srows=[];sm={}
 for name in NAMES:
  sm[name]={}
  for n in [96,97,99,102,105,108]:
   k=np.arange(1,5);p=params[n];f=p['f0']*k*np.sqrt(1+p['B']*k*k)
   for v in VEL:sm[name][f'{n}:{v}']=c.attack(b.load(str(OUT/'attack'/name/f'n{n}v{v}.wav'),stereo=True),f,.5,.5)
 for old in oldrows:
  r={k:old[k] for k in ['source','note','velocity','k','target']};key=f"{r['note']}:{r['velocity']}";i=r['k']-2
  r['values']={n:sm[n][key]['partials'][i] for n in NAMES};r['snr']={n:sm[n][key]['snr'][i] for n in NAMES}
  assert all(v is not None for v in r['values'].values());srows.append(r)
 targets=[]
 for n,v,k in [(102,100,3),(99,45,3),(97,45,2),(108,45,2),(108,56,2),(108,100,2)]:
  key=f'{n}:{v}';rr=[a[key]['partials'][k-2] for a in refs.values() if key in a and a[key]['partials'][k-2] is not None]
  targets.append(dict(note=n,velocity=v,k=k,target_mean=np.mean(rr),target_sd=np.std(rr),sources=len(rr),values={name:models[name][key]['partials'][k-2] for name in NAMES}))
 dense={};dips={}
 for name in NAMES:
  dense[name]={};dips[name]={}
  for n in DENSE:
   m=json.load(open(OUT/'dense'/name/f'n{n}.json'));dense[name].update(m)
   vals=[[m[f'{n}:{v}']['partials'][i] for v in range(28,128)] for i in range(3)]
   dips[name][n]=[max(np.maximum.accumulate(y)-y) if all(v is not None for v in y) else None for y in vals]
 common=set.intersection(*[set(json.load(open(RESULT/f'full-{n}.json'))['z']) for n in NAMES])
 full_scores={}
 for n in NAMES:
  z=json.load(open(RESULT/f'full-{n}.json'))['z'];full_scores[n]=dict(native_components=len(z),fixed_components=len(common),fixed_summary=b.summary({k:z[k] for k in common}))
 dump(RESULT/'summary.json',dict(attack=scores([r for r in rows if r['note']>=96]),transition=scores([r for r in rows if r['note']<96]),sustain=scores(srows),targets=targets,dips=dips,full_fixed=full_scores,attack_rows=rows,sustain_rows=srows))
 dump(RESULT/'dense-models.json',dense)
 print(json.dumps(c.clean(dict(attack=scores([r for r in rows if r['note']>=96]),sustain=scores(srows),targets=targets)),indent=1),flush=True)

if __name__=='__main__':
 p=prepare();render(p);full();score(p)
