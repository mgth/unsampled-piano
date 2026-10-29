from pathlib import Path
import sys,json,importlib.util,hashlib,subprocess,tomllib
import numpy as np
R=Path(__file__).resolve().parents[2];P=R/'bench/upstream-4a00f00';O=R/'out/upstream-4a00f00';BIN=O/'bin';O.mkdir(parents=True,exist_ok=True)
spec=importlib.util.spec_from_file_location('player_support',R/'bench/rayleigh-c8-player/support.py');s=importlib.util.module_from_spec(spec);spec.loader.exec_module(s)
c=s.c;b=c.b
GEOMS=['close','player_actual'];NOTES=list(range(93,109));VEL=[45,56,100];DENSE=[97,99,102,108]
CONTACT=s.CONTACT
save=c.save
run=c.run
def sha(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def preset_entry(n):return CONTACT.get(str(n),s.prev.entry(n))
def flat(d,p=''):
 out={}
 for k,v in d.items():
  if isinstance(v,dict):out.update(flat(v,p+k+'.'))
  else:out[p+k]=v
 return out
def changes(a,z):
 a=flat(a);z=flat(z);return {k:dict(previous=a.get(k),current=z.get(k)) for k in sorted(a.keys()|z.keys()) if a.get(k)!=z.get(k)}
def apply_contact(base,n,path):
 e=preset_entry(n);c.write_preset(path,base,**e['params']);path.write_text(s.prev.patch(path.read_text(),e['extra']));return path
