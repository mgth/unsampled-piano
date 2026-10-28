from support import *

def main():
 source=sys.argv[1];names=sys.argv[2:];rows=json.load(open(P/(source+'-results.json')));jobs=[{k:x[k] for k in ['name','entry']} for x in rows if x['name'] in names];assert len(jobs)==len(names);run_jobs(jobs,'dense-'+source,vel=range(28,128),root=O/'dense')
if __name__=='__main__':main()
