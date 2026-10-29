"""Full keyboard and release validation of the assembled candidate bundle."""
import keyboard_common as c
import keyboard_diagnose as d
from concurrent.futures import ThreadPoolExecutor
import json
from scipy.io import wavfile
import numpy as np

def main():
 selection=json.loads((c.O/'selection.json').read_text());notes=set(map(int,selection['selected']))
 def job(ns):
  out=c.O/f'candidate_{ns[0]}'
  c.render(c.O/'prototype_keyboard/bundle.toml',out,ns)
  return {f'{n}|{v}':c.measure(out/f'n{n}v{v}.wav',n) for n in ns for v in c.VELOCITIES}
 with ThreadPoolExecutor(2) as pool:
  a=pool.submit(job,c.NOTES[:44]);b=pool.submit(job,c.NOTES[44:]);candidate=a.result()|b.result()
 c.save(c.O/'candidate.json',candidate)
 base=json.loads((c.O/'current.json').read_text());checks={}
 refs=json.loads((c.O/'references.json').read_text())['notes']
 def rms(w,start,end):return np.sqrt(np.mean(w[int(start*48000):int(end*48000)].astype(float)**2))
 for k,row in candidate.items():
  n=int(k.split('|')[0]);sr,w=wavfile.read(row['path']);sr0,w0=wavfile.read(base[k]['path'])
  assert sr==sr0==48000 and np.isfinite(w).all() and len(w)==len(w0)
  check=dict(identical=row['sha256']==base[k]['sha256'],peak=row['peak'])
  assert row['peak']<1,k
  if n not in notes:
   a=np.array(base[k]['levels'],float);b=np.array(row['levels'],float)
   mask=np.isfinite(a)&(a>=np.nanmax(a[0])-45)
   assert np.isfinite(b[mask]).all(),k
   check['audible_partial_max_delta']=float(np.max(abs(a[mask]-b[mask])))
   assert check['audible_partial_max_delta']<.3,k
   assert d.assess(n,base[k],refs[k])['status']==d.assess(n,row,refs[k])['status'],k
  else:
   check['release_delta']=float(20*np.log10((rms(w,4.3,4.5)+1e-20)/(rms(w0,4.3,4.5)+1e-20)))
   check['release_drop']=float(20*np.log10((rms(w,4.3,4.5)+1e-20)/(rms(w,3.8,4)+1e-20)))
  checks[k]=check
 c.save(c.O/'validation.json',dict(checks=checks,bundle={p.name:c.sha(p) for p in (c.O/'prototype_keyboard').iterdir() if p.is_file()},renderer_sha256=c.sha(c.EXE)))
 print('validated',len(checks),'renders; max control partial change',max(z.get('audible_partial_max_delta',0) for z in checks.values()),flush=True)
 print('release increases >3dB',[(k,round(z['release_delta'],2)) for k,z in checks.items() if z.get('release_delta',0)>3],flush=True)

if __name__=='__main__':main()
