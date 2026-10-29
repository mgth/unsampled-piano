from study import *
import shutil

def main():
 exe=O/'bin/experimental_contact_probe';exe.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(R/'target/release/examples/experimental_contact_probe',exe)
 names=['control','hysteresis_0.2','freq375_only','freq375_h0.2','strike0.108_h0.15','hysteresis_0.15']
 def job(item):
  name,v=item;d=O/name/'contact';d.mkdir(parents=True,exist_ok=True)
  r=run([exe,O/name/'bundle.toml',102,v,d/f'v{v}.csv'])
  stats={k:float(x) for k,x in re.findall(r'(\w+)=([-\d.eE+]+)',r.stderr)}
  return name,v,stats
 result={n:{} for n in names}
 with ThreadPoolExecutor(4) as ex:
  for n,v,s in ex.map(job,[(n,v) for n in names for v in VEL]):result[n][v]=s
 save(P/'contact-stats.json',result)
 print(json.dumps(result,indent=1))
if __name__=='__main__':main()
