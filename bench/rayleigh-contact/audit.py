"""Audit exact reference support, unchanged keys, numerical validity and provenance."""
from common import *
import hashlib,subprocess,importlib.util
spec=importlib.util.spec_from_file_location('contact_validation',P/'validate.py');val=importlib.util.module_from_spec(spec);spec.loader.exec_module(val)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()

def main():
 summary=json.load(open(P/'validation-summary.json'));old=json.load(open(R/'bench/main-revalidation/summary.json'));ids=lambda rows:{tuple(r[k] for k in ['source','note','velocity','k']) for r in rows}
 for key,count in [('attack_rows',459),('sustain_rows',71)]:
  assert len(summary[key])==len(ids(summary[key]))==count
  assert ids(summary[key])==ids(old[key])
  assert all(np.isfinite(v) for r in summary[key] for v in r['values'].values())
 untouched={};hashes={};peak=0
 for name in val.NAMES:
  counts={}
  for stage,notes,vels in [('attack',range(93,109),[45,56,100]),('dense',[97,99,102,108],range(28,128))]:
   count=0
   for n in notes:
    for v in vels:
     path=val.file(stage,name,n,v);sr,x=wavfile.read(path);assert sr==48000 and x.ndim==2 and x.shape[1]==2 and np.isfinite(x).all();peak=max(peak,float(np.max(abs(x))));hashes[str(path.relative_to(R))]=sha(path)
     if name in ['contact','contact_pp'] and n not in [102,108]:assert path.read_bytes()==val.file(stage,'driven',n,v).read_bytes();count+=1
   counts[stage]=count
  untouched[name]=counts
 checks=[json.load(open(f)) for f in (O/'validation/long').glob('*/n*/checks.json')];worst=max(r['rms_error_db'] for d in checks for r in d.values());assert worst < -90
 for name in ['contact','contact_pp']:
  assert untouched[name]=={'attack':42,'dense':200}
  for n in ['102','108']:
   for new,base in zip(summary['dips'][name][n],summary['dips']['balanced'][n]):
    if new is not None:assert new<=base
 assert not subprocess.check_output(['git','diff','6343657','--','crates','presets'],cwd=R).strip()
 inputs=[R/'bench/main-revalidation/effective.toml',R/'bench/main-revalidation/references.json',R/'bench/main-revalidation/note-parameters.json',a.BIN,O/'bin/contact_stages',a.O/'kernel5/responses.npz',a.O/'kernel-player/responses.npz']
 inputs+=list(P.glob('*.py'))+list(P.glob('*.json'))
 inputs=[p for p in inputs if p.name!='audit.json']
 c.save(P/'audit.json',dict(base_commit='6343657',branch=subprocess.check_output(['git','branch','--show-current'],cwd=R,text=True).strip(),reference_counts=dict(attack=459,sustain=71),untouched_vs_integral=untouched,worst_long_reconstruction_rms_db=worst,max_audio_peak=peak,production_and_defaults_unchanged=True,local_dips_not_worse=True,global_nonregression=False,scope='Local dip guard only: C8 energy/mf and inherited integral regressions on other keys remain. Same references used for tuning and reporting, no held-out validation.',input_sha256={str(p.relative_to(R)):sha(p) for p in inputs},wav_sha256=hashes))
 print('459 attack + 71 sustain IDs fixed; 1792 stereo WAVs finite; unchanged keys verified; local dips pass; global non-regression NOT achieved; defaults unchanged; worst reconstruction',worst)
if __name__=='__main__':main()
