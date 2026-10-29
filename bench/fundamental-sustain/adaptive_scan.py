"""Re-measure formerly insufficient keys and neighbouring real recordings."""
import keyboard_common as c
import adaptive_h1 as a
from concurrent.futures import ThreadPoolExecutor
import json
O=c.O/'adaptive';O.mkdir(exist_ok=True)
FOCUS=[21,22,23,24,25,26,28,31,107]
NEIGHBOURS=sorted(set(m for n in FOCUS for m in range(max(21,n-6),min(108,n+6)+1)))

def main():
 old=json.loads((c.O/'references.json').read_text())['notes']
 base=json.loads((c.O/'current.json').read_text());previous=json.loads((c.O/'candidate.json').read_text())
 jobs=[]
 for n in NEIGHBOURS:
  for v in c.VELOCITIES:
   k=f'{n}|{v}'
   for name,r in old[k].items():
    if name=='main' and n not in FOCUS:continue
    jobs.append((k,name,r['path']))
   if n in FOCUS:
    jobs.extend([(k,'current',base[k]['path']),(k,'previous',previous[k]['path'])])
 def job(item):
  k,name,path=item;n=int(k.split('|')[0]);r=a.measure(path,n)
  r.update(path=path,sha256=c.sha(path));return k,name,r
 rows={}
 with ThreadPoolExecutor(3) as pool:
  for i,(k,name,r) in enumerate(pool.map(job,jobs)):
   rows.setdefault(k,{})[name]=r
   if i%100==0:print('measured',i,'/',len(jobs),flush=True)
 c.save(O/'measurements.json',dict(focus=FOCUS,neighbours=NEIGHBOURS,rows=rows,method=dict(times_bass=a.TIMES,times_treble=[.03]+a.TIMES,cycles=12,min_window=.12,attack_window_treble=.06,window='Blackman-Harris',padding=8,snr=a.SNR_DB,leakage_floor_db=-85,bass_pitch='H2–H5 fit with inharmonicity term; first 0.1–1.3 s')))
 for n in FOCUS:
  for v in c.VELOCITIES:
   k=f'{n}|{v}';print(k,'model',rows[k]['current']['curve'],'valid refs',[nm for nm,r in rows[k].items() if nm not in ['main','current','previous'] and all(z is not None for z in r['curve'])],flush=True)

if __name__=='__main__':main()
