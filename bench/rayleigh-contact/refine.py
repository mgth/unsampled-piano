"""Coupled felt/shank and finite contact-width screen after one-factor isolation."""
from common import *
from concurrent.futures import ThreadPoolExecutor
import itertools

def main():
 jobs=[]
 for q,p,h,f in itertools.product([.5,1.,1.6,2.5],[-1,0],[.1,.3,.6],[300,600]):
  e=json.loads(json.dumps(entry(102)));e['params'].update(q=q,p=p,h=h);e['extra']['shank_freq']=f;jobs.append((102,f'r{len(jobs):03}',e))
 for q,p,w in itertools.product([.1,.2,.35],[-1.5,-.5],[1,2,3]):
  e=json.loads(json.dumps(entry(108)));e['params'].update(q=q,p=p,w=w);e['extra'].update(shank_share=.45,shank_freq=300,shank_loss=2.);jobs.append((108,f'r{len(jobs):03}',e))
 for n in [102,108]:
  for mass,w in itertools.product([.5,1.5,2],[1,2]):
   e=json.loads(json.dumps(entry(n)));e['params'].update(q=1 if n==102 else .25,p=0 if n==102 else -.5,w=w);e['extra'].update(shank_share=.45,shank_freq=300,shank_loss=2.,hammer_mass_scale=mass);jobs.append((n,f'r{len(jobs):03}',e))
 c.save(P/'refine-design.json',dict(cases=[dict(note=n,name=name,entry=e) for n,name,e in jobs],velocities=VEL))
 def one(job):
  n,name,e=job;m=render(n,name,e);s=score(n,m);r=dict(note=n,name=name,entry=e,metrics=m,**s);print(n,name,round(s['mae'],2),s['dips'],s['signed'],flush=True);return r
 with ThreadPoolExecutor(5) as ex:r=list(ex.map(one,jobs))
 c.save(P/'refine-results.json',r)
if __name__=='__main__':main()
