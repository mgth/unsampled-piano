"""Whole-compass long-note bench for upstream default and experimental UI engine."""
from common import *
from concurrent.futures import ThreadPoolExecutor
import shutil
NAMES=['default','integral']
def render(x):
 name,part=x;ns=b.KEYS[part::4];out=O/'full'/name/'notes';out.mkdir(parents=True,exist_ok=True)
 if '--reuse' not in sys.argv:
  if name=='default':cmd=[BIN/'contact_batch',O/'close/base.toml',out,','.join(map(str,ns)),'45,56,100',20,.5]
  else:cmd=[BIN/'rayleigh_live',O/'close/bundle.toml',out,','.join(map(str,ns)),'45,56,100',20,.5,512,1]
  run(cmd)
  for n in ns:
   for layer,v in b.LAYERS.items():(out/f'n{n}v{v}.wav').replace(out/f'{b.name(n)}v{layer}.wav')
 print('long',name,part,flush=True)
def main():
 with ThreadPoolExecutor(4) as ex:list(ex.map(render,[(name,part) for name in NAMES for part in range(4)]))
 refs=json.load(open(R/'bench/references.json'));assert all(x['analysis_version']==b.ANALYSIS_VERSION for x in refs['sources'].values())
 old=json.load(open(R/'bench/main-revalidation/full-base.json'));assert old['analysis_version']==b.ANALYSIS_VERSION
 results={};support=set(old['z'])
 for name in NAMES:
  out=O/'full'/name;run([BIN/'piano-survey',out/'notes','--out',out/'survey.csv','--channel-power'])
  metrics=b.measure_set(str(out/'notes'),str(out/'survey.csv'));z=b.score(metrics,refs);missing=sorted(support-set(z));added=sorted(set(z)-support)
  result=dict(analysis_version=b.ANALYSIS_VERSION,metrics=metrics,z=z,summary=b.summary(z),fixed_summary=b.summary({k:z[k] for k in support}) if not missing else None,missing=missing,added=added,fixed_components=len(support),rendered_notes=b.KEYS,layers=b.LAYERS)
  save(P/f'full-{name}.json',result);b.report('upstream-'+name,z,{},dict(commit='c0c91ce corrected bench',z=old['z']),str(P/f'full-{name}.md'));results[name]={k:v for k,v in result.items() if k not in ['metrics','z']};print(name,results[name],flush=True)
 save(P/'full-summary.json',dict(previous=b.summary(old['z']),profiles=results));assert all(not x['missing'] for x in results.values()),'lost full-bench support; do not silently intersect'
if __name__=='__main__':main()
