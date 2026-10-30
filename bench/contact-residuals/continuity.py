from search import *
from rank import cost
vels=sorted(set(range(28,128,2))|{45,127})
models=json.load(open(OUT/'validation-models.json'))
for name,s in [('base',c.DEFAULT),('guarded',c.interpolate(GUARD,97))]:models[name].update(render('control-'+name+'97',97,s,vels=vels))
c.save(OUT/'validation-models.json',models)
result={}
for name,a in models.items():
 result[name]={}
 for n in [97,99,102,108]:
  if not all(f'{n}:{v}' in a for v in vels):continue
  parts=[[a[f'{n}:{v}']['partials'][i] for v in vels] for i in range(3)]
  out=[]
  for y in parts:
   if any(x is None for x in y):out.append(None);continue
   drop=np.maximum.accumulate(y)-y;j=int(np.argmax(drop));i=int(np.argmax(y[:j+1]));out.append(dict(drop=float(drop[j]),from_velocity=vels[i],to_velocity=vels[j],minimum=min(y)))
  result[name][n]=out
c.save(OUT/'continuity.json',result);print(json.dumps(result,indent=1))
