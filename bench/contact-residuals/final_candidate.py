from search import *
grid=json.load(open(OUT/'refine-grid.json'));height=json.load(open(OUT/'height-grid.json'))
configs=json.load(open(OUT/'candidates.json'));configs['regular']={'97':grid['n97a056dense'],'99':height['n99-height-5e-05'],'102':grid['fine102-11']};c.save(OUT/'candidates.json',configs)
vels=list(range(28,128)) # all 100 velocities; odd values not used in coarse refinement
models=json.load(open(OUT/'validation-models.json'));models['regular']=MODELS['guarded'].copy()
def one(j):
 n,e=j;return n,render('regular-'+n,int(n),e['params'],e.get('extra',{}),vels)
with ThreadPoolExecutor(4) as pool:
 for n,a in pool.map(one,configs['regular'].items()):models['regular'].update(a)
# Re-render unmodified Do8 with exact same profile; compare all 100 velocities.
s=c.interpolate(GUARD,108);a=render('regular-108',108,s,vels=vels);models['regular'].update(a)
old=MODELS['guarded'].copy();old.update(json.load(open(ROOT/'bench/contact-voicing/odd-check.json'))['guarded'])
err=max(abs(a[f'108:{v}']['partials'][0]-old[f'108:{v}']['partials'][0]) for v in vels)
c.save(OUT/'c8-identity.json',dict(velocities=vels,maximum_partial_delta_db=err,parameters_unchanged=s))
c.save(OUT/'validation-models.json',models)
print('Do8 maximum delta',err)
