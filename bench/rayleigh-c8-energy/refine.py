"""Combine the energy-restoring shank loss with felt and height controls."""
from support import *
import itertools

def main():
 jobs=[]
 def add(q,p,h,d,loss):
  e=deepcopy(SEED);e['params'].update(q=q,p=p,h=h,d=d);e['extra']['shank_loss']=loss;jobs.append(dict(name=f'r{len(jobs):03}',entry=e))
 for q,p,h,d in itertools.product([.3,.4,.5],[-1.5,-1.,-.5],[.3,.6],[15e-6,25e-6]):add(q,p,h,d,.1)
 for loss,p,d in itertools.product([.05,.2,.3],[-1.5,-1.],[15e-6,25e-6]):add(.4,p,.6,d,loss)
 run_jobs(jobs,'refine')
if __name__=='__main__':main()
