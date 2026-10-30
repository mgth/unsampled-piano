from search import *
from rank import cost
GRID=json.load(open(OUT/'grid.json'));HEIGHT=json.load(open(OUT/'height-grid.json'));GEO=json.load(open(OUT/'geometry-grid.json'))
# Frozen candidates before velocity and held-source checks. Only named keys differ.
configs={
 'heights':{'97':HEIGHT['n97-height-6e-05'],'99':HEIGHT['n99-height-5e-05']},
 'felt':{'97':GRID['n97a077'],'99':GRID['n99a075']},
 'geometry':{'97':GRID['n97a077'],'99':GRID['n99a075'],'102':GEO['geo021']},
 'geometry_heights':{'97':HEIGHT['n97-height-6e-05'],'99':HEIGHT['n99-height-5e-05'],'102':GEO['geo021']},
}
c.save(OUT/'candidates.json',configs)
vels=sorted(set(range(28,128,2))|{45,127})
uniq={}
for cn,cs in configs.items():
 for n,e in cs.items():uniq[json.dumps(e,sort_keys=True)]=(n,e)
def one(j):
 key,(n,e)=j;tag=hashlib.sha256(key.encode()).hexdigest()[:12];a=render('dense-'+tag,int(n),e['params'],e.get('extra',{}),vels);return key,a
with ThreadPoolExecutor(6) as pool:measurements=dict(pool.map(one,uniq.items()))
models={n:MODELS[n].copy() for n in ['base','guarded']}
for name,entries in configs.items():
 models[name]=MODELS['guarded'].copy()
 for n,e in entries.items():models[name].update(measurements[json.dumps(e,sort_keys=True)])
c.save(OUT/'validation-models.json',models)
rows=c.fixed_rows(models,REFS)
scores={}
for split,pred in {
 'all_top':lambda r:r['note']>=96,
 'target_keys':lambda r:r['note'] in [97,99,102],
 'held_sources':lambda r:r['note']>=96 and r['source'] in c.HOLDOUT,
 'target_held_sources':lambda r:r['note'] in [97,99,102] and r['source'] in c.HOLDOUT,
 'unmodified_keys':lambda r:r['note']>=96 and r['note'] not in [97,99,102],
 'pp':lambda r:r['note']>=96 and r['velocity']==45,
}.items():
 sub=[r for r in rows if pred(r)];scores[split]={}
 for name in models:
  errs={s:[r['values'][name]-r['target'] for r in sub if r['source']==s] for s in REFS};errs={s:v for s,v in errs.items() if v}
  scores[split][name]=dict(mae=np.mean([np.mean(np.abs(e)) for e in errs.values()]),bias=np.mean([np.mean(e) for e in errs.values()]),rows=len(sub),sources=len(errs),low_snr=sum(r['snr'][name]<12 for r in sub))
dips={}
for name,entries in configs.items():
 dips[name]={}
 for n in entries:
  a=models[name];levels=np.array([a[f'{n}:{v}']['level'] for v in vels]);parts=[[a[f'{n}:{v}']['partials'][i] for v in vels] for i in range(3)]
  dips[name][n]=dict(level_dip=max(np.maximum.accumulate(levels)-levels),part_dips=[max(np.maximum.accumulate(y)-y) if all(x is not None for x in y) else None for y in parts],partials=parts,velocities=vels)
c.save(OUT/'summary.json',dict(scores=scores,dips=dips,coverage_rows=rows,analysis_version='contact-physical-peaks-v1',reference_protocol=c.b.ANALYSIS_VERSION))
print(json.dumps(c.clean(scores),indent=1));print('DIPS',[(k,{n:(v['level_dip'],v['part_dips']) for n,v in d.items()}) for k,d in dips.items()])
