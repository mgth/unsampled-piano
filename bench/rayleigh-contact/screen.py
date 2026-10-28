from common import *
from concurrent.futures import ThreadPoolExecutor
import itertools

def main():
 jobs=[]
 for n in [102,108]:
  jobs.append((n,'control',entry(n)));jobs.append((n,'main',{'params':c.DEFAULT,'extra':{}}))
  qs=[.25,.4,.65,1.,1.6] if n==102 else [.15,.25,.45,.75]
  ps=[-2,-1,0] if n==102 else [-2,-1.25,-.5,0]
  for j,(q,p) in enumerate(itertools.product(qs,ps)):
   e=json.loads(json.dumps(entry(n)));e['params'].update(q=q,p=p);jobs.append((n,f'felt{j:02}',e))
  for j,extra in enumerate([{'shank_share':.45,'shank_freq':150,'shank_loss':2.},{'shank_share':.45,'shank_freq':300,'shank_loss':2.},{'shank_share':.3,'shank_freq':150,'shank_loss':2.},{'shank_share':0},{'strike_ratio':.085},{'strike_ratio':.095},{'strike_ratio':.12},{'strike_ratio':.14} ]):
   e=json.loads(json.dumps(entry(n)));e['extra'].update(extra);jobs.append((n,f'mechanics{j:02}',e))
  for j,d in enumerate([0,.000025,.00005,.0001]):
   e=json.loads(json.dumps(entry(n)));e['params']['d']=d;jobs.append((n,f'height{j:02}',e))
 c.save(P/'screen-design.json',dict(cases=[dict(note=n,name=name,entry=e) for n,name,e in jobs],velocities=VEL,policy='Full radiator, fixed mechanics except stated contact fields, same source rows. No output EQ or velocity-dependent mix.'))
 def one(job):
  n,name,e=job;m=render(n,name,e);s=score(n,m);r=dict(note=n,name=name,entry=e,metrics=m,**s);print(n,name,round(s['mae'],2),s['dips'],s['signed'],flush=True);return r
 with ThreadPoolExecutor(5) as ex:results=list(ex.map(one,jobs))
 c.save(P/'screen-results.json',results)
if __name__=='__main__':main()
