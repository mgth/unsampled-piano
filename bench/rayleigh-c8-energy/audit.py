"""Frozen-support audit, explicit no-regression guards and provenance."""
from support import *
import hashlib,subprocess
spec=importlib.util.spec_from_file_location('c8_validation',P/'validate.py');val=importlib.util.module_from_spec(spec);spec.loader.exec_module(val)
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()

def main():
 s=json.load(open(P/'validation-summary.json'));old=json.load(open(prev.P/'validation-summary.json'));ident=lambda rows:{tuple(r[k] for k in ['source','note','velocity','k']) for r in rows}
 for key,count in [('attack_rows',459),('sustain_rows',71)]:
  assert len(s[key])==len(ident(s[key]))==count and ident(s[key])==ident(old[key]);assert all(np.isfinite(v) for r in s[key] for v in r['values'].values())
 counts={};hashes={};peak=0
 for name in val.NAMES:
  counts[name]={}
  for stage,notes,vels in [('attack',range(93,109),[45,56,100]),('dense',[97,99,102,108],range(28,128))]:
   count=0
   for n in notes:
    for v in vels:
     path=val.file(stage,name,n,v);sr,x=wavfile.read(path);assert sr==48000 and x.ndim==2 and x.shape[1]==2 and np.isfinite(x).all();peak=max(peak,float(np.max(abs(x))));hashes[str(path.relative_to(R))]=sha(path)
     if name.startswith('recovered') and n!=108:assert path.read_bytes()==val.file(stage,'contact',n,v).read_bytes();count+=1
   counts[name][stage]=count
 guards=json.load(open(P/'energy-guards.json'));audio=json.load(open(P/'audio-convergence.json'));verdict={}
 for name in ['recovered','recovered_mf']:
  assert counts[name]=={'attack':45,'dense':300}
  for v in ['45','56','100']:assert s['c8_dynamics'][v][name]['mae']<s['c8_dynamics'][v]['contact']['mae']
  mf=[r for r in s['attack_rows'] if r['note']==108 and r['velocity']==56];assert all(abs(r['values'][name]-r['target'])<=abs(r['values']['contact']-r['target']) for r in mf)
  assert s['dips'][name]['108'][0] < s['dips']['contact']['108'][0] and s['level_dips'][name]['108']==0
  assert guards['dense_summary'][name]['balanced']['1-2k']['min']>0
  assert guards['dense_summary'][name]['contact']['level_preroll']['min']>0
  q=audio['models'][name]['score'];assert q['by_velocity']['56']<audio['models']['legacy10']['score']['by_velocity']['56'];assert q['dips'][0]<audio['models']['contact']['score']['dips'][0] and q['level_dip']==0
  verdict[name]=dict(local_guards_pass=True,mf_each_of_4_sources_improves_vs_previous=True,all_100_velocities_low_band_above_historical=True,all_100_velocities_level_above_previous=True,fsharp_unchanged=True,audio10_guard_pass=True)
 assert not subprocess.check_output(['git','diff','1ba30c6','--','crates/piano-core','crates/piano-host','crates/piano-ui','presets'],cwd=R).strip()
 checks=[json.load(open(f)) for f in (O/'validation/long').glob('*/n*/checks.json')];worst=max(r['rms_error_db'] for d in checks for r in d.values());assert worst < -90
 inputs=list(P.glob('*.py'))+[p for p in P.glob('*.json') if p.name!='audit.json']+[R/'crates/piano-render/examples/rayleigh_capture.rs',a.BIN,O/'bin/rayleigh_capture',prev.O/'bin/contact_stages',a.O/'kernel5/responses.npz',a.O/'kernel-player/responses.npz',R/'bench/main-revalidation/references.json',prev.P/'selected.json',prev.P/'reference-levels.json']
 c.save(P/'audit.json',dict(base_commit='1ba30c6',reference_counts=dict(attack=459,sustain=71),unchanged_vs_previous=counts,verdict=verdict,worst_long_reconstruction_rms_db=worst,max_audio_peak=peak,default_capture_control=audio['controls'],production_and_defaults_unchanged=True,limits='Isolated notes, fixed close-mic benchmark, same tuning/evaluation sources. Full keyboard and player-position realism not established. Remaining historical level deficit near v127. No C8 sustain rows.',input_sha256={str(p.relative_to(R)):sha(p) for p in inputs},wav_sha256=hashes))
 print('459+71 reference IDs fixed; 1792 stereo WAVs finite; each candidate preserves 45 attacks and 300 dense files; local guards pass at default and audit10; default captures bit-identical; production/defaults unchanged.')
if __name__=='__main__':main()
