"""All 88 keys, three velocities: current app, main and exact-key real pianos."""
import keyboard_common as c
from concurrent.futures import ThreadPoolExecutor
import json
from pathlib import Path

def references():
 sources=json.loads(Path(c.s.b.REFS).read_text())['sources']
 rows={}
 for n in c.NOTES:
  for layer,v in c.s.b.LAYERS.items():
   paths={nm:c.s.b.find(str(c.R/src['dir']),n,layer) for nm,src in sources.items() if src['role']=='target'}
   paths['main']=next((p for d in ['notes','harmonic_notes'] if (p:=c.s.b.find(str(c.R/'out/bench/harmonic_local'/d),n,layer))),None)
   assert paths['main']
   rows[f'{n}|{v}']={nm:c.measure(path,n) for nm,path in paths.items() if path}
  if n%12==0:print('references through',n,flush=True)
 c.save(c.O/'references.json',dict(sources={nm:{k:src[k] for k in ['description','dir']} for nm,src in sources.items() if src['role']=='target'},notes=rows))
 return rows

def current(notes):
 out=c.O/f'current_{notes[0]}'
 c.render(c.SOURCE/'bundle.toml',out,notes)
 return {f'{n}|{v}':c.measure(out/f'n{n}v{v}.wav',n) for n in notes for v in c.VELOCITIES}

if __name__=='__main__':
 with ThreadPoolExecutor(3) as pool:
  ref=pool.submit(references)
  a=pool.submit(current,c.NOTES[:44]);b=pool.submit(current,c.NOTES[44:])
  result=a.result()|b.result();ref.result()
 c.save(c.O/'current.json',result)
 print('scan complete',len(result),flush=True)
