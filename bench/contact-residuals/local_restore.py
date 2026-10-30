from search import *
configs=json.load(open(OUT/'candidates.json'));configs['local']=dict(configs['regular']);configs['local']['97']=dict(note=97,params=c.DEFAULT);c.save(OUT/'candidates.json',configs)
models=json.load(open(OUT/'validation-models.json'));models['local']=dict(models['regular']);a=render('local-97',97,c.DEFAULT,vels=list(range(28,128)));models['local'].update(a);c.save(OUT/'validation-models.json',models)
# Identical settings on all standard bench notes (97 is the supplemental key).
import shutil
shutil.copytree(OUT/'full/regular',OUT/'full/local',symlinks=True,dirs_exist_ok=True)
d=OUT/'full/local';preset(d/'n97.toml',c.DEFAULT)
for v in [45,56,100]:shutil.copy2(OUT/'full/base/supplement'/f'n97v{v}.wav',d/'supplement'/f'n97v{v}.wav')
a=json.load(open(d/'result.json'));a['profile']='local';a['identical_90_note_render']='regular';c.save(d/'result.json',a)
# Extend fixed sustained support without rereading or recensoring references.
a=json.load(open(OUT/'sustain-check.json'));a['models']['local']=dict(a['models']['regular'])
for k,v in a['models']['base'].items():
 if k.startswith('97:'):a['models']['local'][k]=v
for r in a['coverage_rows']:
 src='base' if r['note']==97 else 'regular';r['values']['local']=r['values'][src];r['snr']['local']=r['snr'][src]
for label,pred in dict(all=lambda r:True,targets=lambda r:r['note'] in [97,99,102],ff=lambda r:r['velocity']==100,p4=lambda r:r['k']==4).items():
 sub=[r for r in a['coverage_rows'] if pred(r)];e={s:[r['values']['local']-r['target'] for r in sub if r['source']==s] for s in REFS};e={s:v for s,v in e.items() if v}
 a['scores'][label]['local']=dict(mae=np.mean([np.mean(np.abs(v)) for v in e.values()]),bias=np.mean([np.mean(v) for v in e.values()]),rows=len(sub),sources=len(e),low_snr=sum(r['snr']['local']<12 for r in sub))
c.save(OUT/'sustain-check.json',a)
from scipy.io import wavfile
for name in ['guarded','local']:
 clips=[]
 for n in [97,99,102,108]:
  for l,v in c.b.LAYERS.items():
   if n==97:p=OUT/'full'/name/'supplement'/f'n97v{v}.wav'
   else:p=(ROOT/'out/contact-voicing/full/guarded/notes' if name=='guarded' else OUT/'full/local/notes')/f'{c.b.name(n)}v{l}.wav'
   clips.extend([c.b.load(str(p),stereo=True)[:57600],np.zeros((14400,2))])
 wavfile.write(OUT/f'listen-{name}.wav',c.b.SR,np.concatenate(clips).astype('float32'))
