from common import *
from concurrent.futures import ThreadPoolExecutor
import itertools

def main():
 jobs=[]
 def add(q=.2,p=-1.5,h=.6,d=4e-5,**extra):
  e=json.loads(json.dumps(entry(108)));e['params'].update(q=q,p=p,h=h,d=d);e['extra'].update(shank_share=.45,shank_freq=300,shank_loss=2.);e['extra'].update(extra);jobs.append((108,f'c{len(jobs):03}',e))
 for q,p,h in itertools.product([.15,.25,.4],[-1.5,-.5],[.1,.6,1.2]):add(q=q,p=p,h=h)
 for ratio in [.09,.105,.12]:add(strike_ratio=ratio)
 for f,loss in itertools.product([150,600],[.5,1.,4.]):add(shank_freq=f,shank_loss=loss)
 for q,d in itertools.product([.2,.35],[0,.00002,.00006,.0001]):add(q=q,d=d)
 c.save(P/'c8-refine-design.json',dict(cases=[dict(note=n,name=name,entry=e) for n,name,e in jobs],velocities=VEL))
 def one(job):
  n,name,e=job;m=render(n,name,e);s=score(n,m);r=dict(note=n,name=name,entry=e,metrics=m,**s);print(n,name,round(s['mae'],2),s['dips'],s['signed'],flush=True);return r
 with ThreadPoolExecutor(5) as ex:r=list(ex.map(one,jobs))
 c.save(P/'c8-refine-results.json',r)
if __name__=='__main__':main()
