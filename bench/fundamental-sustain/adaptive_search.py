"""Search newly measurable H1 alerts, preserving the preceding candidate."""
import json,math
import numpy as np
from concurrent.futures import ThreadPoolExecutor
import keyboard_common as c
import adaptive_h1 as a
import adaptive_references as ar
from adaptive_scan import O,FOCUS
from keyboard_search import PAIRS

def score(n,rows,refs,base):
 excess=[];mean=[]
 for v in c.VELOCITIES:
  k=f'{n}|{v}'
  if ar.assess(base[k]['previous'],refs[k])['status']=='insufficient':continue
  x=ar.assess(rows[k]['adaptive'],refs[k])
  if x['status']=='insufficient':return float('inf')
  excess.extend(x['excess']);mean.extend(x['mean_delta'])
 return float(np.mean(np.square(excess))+.025*np.mean(np.square(mean)))

def guard(n,rows,refs,base,old):
 for v in c.VELOCITIES:
  k=f'{n}|{v}';new=rows[k]['adaptive'];previous=base[k]['previous']
  if any(z is None for z in new['curve']):return 'H1 incomplet'
  if abs(new['windows'][0]['level']-previous['windows'][0]['level'])>(1 if n>=40 else 3):return 'niveau initial H1 modifié au-delà du seuil'
  aa=np.array(old[k]['levels'],float);bb=np.array(rows[k]['legacy']['levels'],float)
  mask=np.isfinite(aa[7,1:])&(aa[7,1:]>=np.nanmax(aa[0])-45)
  if not np.isfinite(bb[7,1:][mask]).all() or np.any(abs(bb[7,1:][mask]-aa[7,1:][mask])>3):return 'H2–H8 >3 dB ou illisible'
  b=ar.assess(previous,refs[k]);z=ar.assess(new,refs[k])
  if b['status']!='insufficient':
   if z['status']=='insufficient':return 'référence perdue'
   for i in range(2):
    if b['directions'][i]=='within' and (z['directions'][i]!='within' or abs(z['value'][i]-b['value'][i])>3):return 'zone correcte dégradée'
 return None

def main():
 references=json.loads((O/'references.json').read_text());refs=references['references'];base=json.loads((O/'measurements.json').read_text())['rows'];old=json.loads((c.O/'candidate.json').read_text())
 notes=sorted({int(k.split('|')[0]) for k,z in references['diagnoses'].items() if z['status'] in ['fast','slow','mixed']})
 preceding=json.loads((c.O/'selection.json').read_text())['selected']
 def job(pair):
  h,f=pair;name=f'h{h}_f{f}';fields={int(n):dict(z) for n,z in preceding.items()}
  for n in notes:fields[n]={'polarization.h_coupling':h,'polarization.cross':math.sqrt(h)*f}
  path=c.bundle(O/f'bundle_{name}',fields);out=O/name;c.render(path,out,notes,hold=1.6,tail=.2)
  rows={}
  for n in notes:
   for v in c.VELOCITIES:
    p=out/f'n{n}v{v}.wav';rows[f'{n}|{v}']=dict(adaptive=a.measure(p,n),legacy=c.measure(p,n))
  c.save(out/'metrics.json',rows);return name,rows
 with ThreadPoolExecutor(3) as pool:results=dict(pool.map(job,PAIRS))
 c.save(O/'search.json',dict(notes=notes,results=results,pairs=PAIRS))
 selected={};outcomes={}
 baseline={k:dict(adaptive=rows['previous']) for k,rows in base.items() if 'previous' in rows}
 for n in notes:
  before=score(n,baseline,refs,base);choices=[];rejected={}
  for name,rows in results.items():
   reason=guard(n,rows,refs,base,old)
   if reason:rejected[name]=reason;continue
   after=score(n,rows,refs,base)
   if after<before*.7:choices.append((after,name))
   else:rejected[name]='gain inférieur à 30 %'
  outcomes[n]=dict(before=before,rejected=rejected)
  if choices:
   after,name=min(choices);h,f=map(float,name[1:].split('_f'))
   selected[n]={'polarization.h_coupling':h,'polarization.cross':math.sqrt(h)*f}
   outcomes[n].update(after=after,variant=name)
  print('selected',n,outcomes[n].get('variant','none'),flush=True)
 c.save(O/'selection.json',dict(selected=selected,outcomes=outcomes,preceding=preceding))
 c.bundle(O/'prototype_adaptive',{int(n):f for n,f in preceding.items()}|selected)

if __name__=='__main__':main()
