"""Targeted combinations after single-field isolation, at both positions."""
from support import *
from concurrent.futures import ThreadPoolExecutor
import itertools

def main():
 geom=sys.argv[1];kernel(geom);jobs=[]
 for q,mass,f,d in itertools.product([.25,.3],[1.05,1.1],[300,350],[10e-6,15e-6]):
  e=deepcopy(ENTRIES['recovered']);e['params'].update(q=q,d=d);e['extra'].update(hammer_mass_scale=mass,shank_freq=f);jobs.append(dict(name=f'c{len(jobs):02}',entry=e))
 for q,share in itertools.product([.2,.25],[.35,.4]):
  e=deepcopy(ENTRIES['recovered']);e['params'].update(q=q,d=10e-6);e['extra']['shank_share']=share;jobs.append(dict(name=f'c{len(jobs):02}',entry=e))
 c.save(P/f'{geom}-coupled-design.json',dict(cases=jobs,velocities=energy.VEL))
 def one(job):
  m=capture(job['entry'],job['name'],geom,vel=energy.VEL,root=O/'screen',keep=False);s=prev.score(108,m) if geom=='close' else {};print(geom,job['name'],s.get('mae'),flush=True);return dict(**job,metrics=m,**s)
 with ThreadPoolExecutor(4) as ex:out=list(ex.map(one,jobs))
 c.save(P/f'{geom}-coupled.json',out)
if __name__=='__main__':main()
