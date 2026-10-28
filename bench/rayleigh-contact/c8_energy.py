from common import *
from concurrent.futures import ThreadPoolExecutor
import itertools

def main():
 jobs=[]
 for q,h,mass in itertools.product([.25,.35,.45],[.1,.3,.6],[.7,1.4]):
  e=json.loads(json.dumps(entry(108)));e['params'].update(q=q,p=-1.5,h=h);e['extra'].update(shank_share=.45,shank_freq=600,shank_loss=1.,hammer_mass_scale=mass);jobs.append((108,f'e{len(jobs):03}',e))
 for share,h in itertools.product([.15,.3],[.1,.3,.6]):
  e=json.loads(json.dumps(entry(108)));e['params'].update(q=.35,p=-1.5,h=h);e['extra'].update(shank_share=share,shank_freq=600,shank_loss=1.);jobs.append((108,f'e{len(jobs):03}',e))
 c.save(P/'c8-energy-design.json',dict(cases=[dict(note=n,name=name,entry=e) for n,name,e in jobs],velocities=VEL))
 def one(job):
  n,name,e=job;m=render(n,name,e);s=score(n,m);r=dict(note=n,name=name,entry=e,metrics=m,**s);print(n,name,round(s['mae'],2),s['dips'],s['signed'],flush=True);return r
 with ThreadPoolExecutor(5) as ex:r=list(ex.map(one,jobs))
 c.save(P/'c8-energy-results.json',r)
if __name__=='__main__':main()
