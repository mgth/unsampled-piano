"""Audit frozen support, spatial controls, retained voicing and file provenance."""
from support import *
import subprocess
spec=importlib.util.spec_from_file_location('player_validation',P/'validate.py');val=importlib.util.module_from_spec(spec);spec.loader.exec_module(val)
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()

def main():
 s=json.load(open(P/'validation-summary.json'));old=json.load(open(energy.P/'validation-summary.json'));ids=lambda rows:{tuple(r[k] for k in ['source','note','velocity','k']) for r in rows}
 for key,count in [('attack_rows',459),('sustain_rows',71)]:
  assert len(s[key])==len(ids(s[key]))==count and ids(s[key])==ids(old[key]);assert all(np.isfinite(v) for r in s[key] for v in r['values'].values())
 hashes={};peak=0;unchanged={}
 for geom in val.GEOMS:
  unchanged[geom]={}
  for name in val.NAMES:
   unchanged[geom][name]={}
   for stage,ns,vs in [('attack',range(93,109),[45,56,100]),('dense',[97,99,102,108],range(28,128))]:
    count=0
    for n in ns:
     for v in vs:
      p=val.file(stage,geom,name,n,v);sr,x=wavfile.read(p);assert sr==48000 and x.ndim==2 and x.shape[1]==2 and np.isfinite(x).all();peak=max(peak,float(np.max(abs(x))));hashes[str(p.relative_to(R))]=sha(p)
      if name!='baseline' and n!=108:assert p.read_bytes()==val.file(stage,geom,'baseline',n,v).read_bytes();count+=1
    unchanged[geom][name][stage]=count
   if name!='baseline':assert unchanged[geom][name]=={'attack':45,'dense':300}
 decision=json.load(open(P/'decision.json'));assert not decision['promoted'] and decision['retained']==CONTACT
 source=prev.tomllib.loads((O/'validation/raw/attack/player_actual/baseline-n108/preset.toml').read_text())
 for k in ['mics','mics_relative','mic_gain_db','mic_pattern','lid_angle']:assert source['soundboard'][k]==ACTUAL[k]
 assert source['room']['enabled']
 source_ref=json.load(open(P/'binaural-band-references.json'));comparison=json.load(open(P/'binaural-band-comparison.json'));rid=lambda r:(r['source'],r['note'],r['velocity'],tuple(r['anchor']));assert {rid(r) for r in source_ref['rows']}=={rid(r) for r in comparison['rows']};assert len(comparison['rows'])==3
 checks=[r['rms_error_db'] for p in O.rglob('checks.json') for r in json.load(open(p)).values()];assert max(checks)<-90
 invariant=json.load(open(P/'path-checks.json'))['action_invariance'];worst=max(x['rms_error_db'] for v in invariant.values() for x in v.values());assert worst < -120
 assert not subprocess.check_output(['git','diff','3a6a3b0','--','crates','presets','bench/rayleigh-c8-energy/selected.json'],cwd=R).strip()
 inputs=list(P.glob('*.py'))+[p for p in P.glob('*.json') if p.name!='audit.json']+[a.BIN,a.O/'bin/rayleigh_kernel',a.O/'kernel5/responses.npz',a.O/'kernel-player/responses.npz',O/'kernel-player35/responses.npz',O/'kernel-player0/responses.npz',R/'presets/listening/Microphones/player.toml',R/'bench/main-revalidation/references.json',energy.P/'selected.json',R/'crates/piano-core/src/action.rs',R/'crates/piano-core/src/instrument.rs']
 c.save(P/'audit.json',dict(base_commit='3a6a3b0',reference_counts=dict(attack=459,sustain=71,auxiliary_binaural_band=3),unchanged=unchanged,production_and_defaults_unchanged=True,retained_contact_unchanged=True,player_preset_fields_verified=True,worst_reconstruction_rms_db=max(checks),worst_action_invariance_rms_db=worst,max_audio_peak=peak,no_candidate_promoted=True,limits='Auxiliary binaural band metric is a local neighbor contrast, not absolute calibration. No eligible pp band row; bare omnis versus dummy head; fixed studio room for BiVib differs from its real room.',input_sha256={str(p.relative_to(R)):sha(p) for p in inputs},wav_sha256=hashes))
 print('459+71 historical identities and 3 auxiliary binaural rows fixed; 3584 stereo WAVs finite; 45 attacks+300 dense WAVs unchanged per candidate and setup; previous contact retained; production/defaults unchanged. Worst reconstruction:',max(checks))
if __name__=='__main__':main()
