"""Independent frequency/causality, grid, room and fixed-coverage checks."""
from audio import *
from guard_search import guarded
import pandas as pd
from scipy.signal import fftconvolve

def main():
 design=json.load(open(R/'bench/bridge-ear/diagnostic-design.json'));freq=design['model_frequencies'];N=IR['driven'].shape[-1]
 dft=np.exp(-2j*np.pi*np.array(freq)[:,None]*np.arange(N)[None,:]/FS)
 x=np.load(O/'kernel-player/responses.npz');reference=pd.read_csv(R/'out/bridge-ear/grid5-driven.csv');reference=reference[reference.band=='all']
 exact=np.array([reference.pl_re+1j*reference.pl_im,reference.pr_re+1j*reference.pr_im]).T
 measured=(x['driven'][14]@dft.T).T
 error_db=20*np.log10(abs(measured/exact));phase=np.angle(measured/exact,deg=True)
 assert np.max(abs(error_db))<.05 and np.max(abs(phase))<.3
 grid=np.load(O/'kernel75/responses.npz');grid_metrics={}
 for n in [97,99,102,108]:
  p=PAR[str(n)];k=np.arange(1,5);ff=p['f0']*k*np.sqrt(1+p['B']*k*k);ff=ff[ff<12000];e=np.exp(-2j*np.pi*ff[:,None]*np.arange(N)[None,:]/FS);region=(n-21)*16//88
  h5=IR['driven'][region]@e.T;h75=grid['driven'][region]@e.T
  grid_metrics[n]=(10*np.log10(np.mean(abs(h5)**2,axis=0)/np.mean(abs(h75)**2,axis=0))).tolist()
 # Explicit dry filtering -> room versus room -> filtering on a long strike.
 folder=O/'validation/capture/attack/n102';dry,new,_=stages(folder,102,100);g=json.load(open(P/'guard-selected.json'))
 z=guarded(dry,new,100,g['maximum'],g['end'],g['start'],g.get('shape','smooth'));explicit=O/'room-check.wav';save_audio(OLD/'presets/balanced/n102.toml',z,explicit)
 _,a=wavfile.read(explicit);_,b=wavfile.read(O/'validation/attack/guarded/n102v100.wav')
 # The endpoint WAVs were cropped at the original duration; their last
 # 64 samples omit room continuation, so only compare the common duration.
 a=a[:len(dry)];b=b[:len(dry)];diff=a.astype(float)-b
 room_error=10*np.log10(np.mean(diff**2)/(np.mean(a**2)+1e-30)+1e-30);assert room_error< -100
 # Repeat the score with the original onset plus the known latency. The
 # ordinary detector differs by at most one sample in the dense set.
 summary=json.load(open(P/'validation-summary.json'));ident=lambda rows:{(r['source'],r['note'],r['velocity'],r['k']) for r in rows}
 previous=json.load(open(R/'bench/main-revalidation/summary.json'));assert ident(summary['attack_rows'])==ident(previous['attack_rows']);assert ident(summary['sustain_rows'])==ident(previous['sustain_rows'])
 c.save(P/'numerical-checks.json',dict(player_frequencies=freq,amplitude_error_db=error_db,phase_error_degrees=phase,grid_5_vs_75_db=grid_metrics,room_commutation_error_db=room_error,room_max_error=float(np.max(abs(diff))),common_attack_rows=len(summary['attack_rows']),common_sustain_rows=len(summary['sustain_rows']),guard_latency=64))
 print(json.dumps(c.clean(dict(error_db=error_db,phase=phase,grid=grid_metrics,room_error_db=room_error)),indent=1),flush=True)
if __name__=='__main__':main()
