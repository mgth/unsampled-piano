from support import *
import itertools

def main():
 jobs=[]
 for q,p,loss in itertools.product([.3,.35],[-1.25,-1.],[.025,.05,.075]):
  e=deepcopy(SEED);e['params'].update(q=q,p=p);e['extra']['shank_loss']=loss;jobs.append(dict(name=f'f{len(jobs):03}',entry=e))
 run_jobs(jobs,'finalists')
if __name__=='__main__':main()
