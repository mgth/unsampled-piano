"""Secondary physical checks when polarisation alone yields no safe gain."""
import keyboard_common as c
import adaptive_h1 as a
import adaptive_search as search
from adaptive_scan import O
import json,math
from concurrent.futures import ThreadPoolExecutor
VARIANTS={
 'loss_half':{'losses.b1_offset':-5.1+math.log(.5),'losses.b2':3e-8},
 'loss_quarter':{'losses.b1_offset':-5.1+math.log(.25),'losses.b2':1.5e-8},
 'winding_half':{'losses.winding':.0001},
 'width3':{'unison.width_treble':3.},'width4':{'unison.width_treble':4.},
 'width56':{'unison.width_treble':5.6},'width58':{'unison.width_treble':5.8},'width59':{'unison.width_treble':5.9},'width6':{'unison.width_treble':6.},'width7':{'unison.width_treble':7.},
 'detune01':{'polarization.detune_cents':.1},'detune06':{'polarization.detune_cents':.6}}

def main():
 selection=json.loads((O/'selection.json').read_text());notes=list(map(int,selection['outcomes']))
 base=json.loads((O/'measurements.json').read_text())['rows'];refs=json.loads((O/'references.json').read_text())['references'];old=json.loads((c.O/'candidate.json').read_text())
 def job(item):
  name,change=item;fields={int(n):dict(z) for n,z in selection['preceding'].items()}
  for n in notes:fields[n]=change
  path=c.bundle(O/f'bundle_{name}',fields);out=O/name;c.render(path,out,notes,hold=1.6,tail=.2)
  rows={f'{n}|{v}':dict(adaptive=a.measure(out/f'n{n}v{v}.wav',n),legacy=c.measure(out/f'n{n}v{v}.wav',n)) for n in notes for v in c.VELOCITIES}
  c.save(out/'metrics.json',rows);return name,rows
 with ThreadPoolExecutor(3) as pool:results=dict(pool.map(job,VARIANTS.items()))
 c.save(O/'extra.json',dict(variants=VARIANTS,results=results))
 for n in notes:
  before=selection['outcomes'][str(n)]['before'];choices=[]
  for name,rows in results.items():
   reason=search.guard(n,rows,refs,base,old);after=search.score(n,rows,refs,base)
   selection['outcomes'][str(n)].setdefault('extra',{})[name]=dict(reason=reason,score=after)
   if not reason and after<before*.7:choices.append((after,name))
  if choices:
   after,name=min(choices);selection['selected'][str(n)]=VARIANTS[name];selection['outcomes'][str(n)].update(after=after,variant=name)
  print(n,selection['outcomes'][str(n)].get('variant','none'),flush=True)
 c.save(O/'selection.json',selection)
 c.bundle(O/'prototype_adaptive',{int(n):f for n,f in selection['preceding'].items()}|{int(n):f for n,f in selection['selected'].items()})

if __name__=='__main__':main()
