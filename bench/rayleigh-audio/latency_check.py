"""Guard against frame-grid bias caused by the explicit 64-sample FIR delay.
Keep the historical level metric, and add the same 1024-sample preroll to ALL
signals before measuring onset level. No gain correction or note normalization.
"""
from audio import *
def main():
 names=['balanced','driven','guarded'];dense={};dips={};raw={};prefix_energy={}
 for name in names:
  dense[name]={};dips[name]={}
  for n in [97,99,102,108]:
   levels=[]
   for v in range(28,128):
    folder=OLD/'dense/balanced' if name=='balanced' else O/'validation/dense'/name
    _,x=wavfile.read(folder/f'n{n}v{v}.wav');level=float(c.b.onset_level(np.pad(x,((1024,0),(0,0)))));dense[name][f'{n}:{v}']=level;levels.append(level)
    if name=='guarded' and n==102 and v>88:prefix_energy[v]=float(10*np.log10(np.sum(x[:64].astype(float)**2)/(np.sum(x.astype(float)**2)+1e-30)+1e-30))
   dips[name][n]=float(max(np.maximum.accumulate(levels)-levels))
 for v in [88,89,100]:
  _,x=wavfile.read(OLD/'dense/balanced'/f'n102v{v}.wav');y=np.pad(x,((64,0),(0,0)))
  raw[v]=dict(unshifted=float(c.b.onset_level(x)),shifted=float(c.b.onset_level(y)),with_preroll=float(c.b.onset_level(np.pad(x,((1024,0),(0,0))))),shifted_with_preroll=float(c.b.onset_level(np.pad(y,((1024,0),(0,0))))))
 assert all(v==0 for d in dips.values() for v in d.values())
 c.save(P/'latency-level-check.json',dict(common_preroll_samples=1024,levels=dense,level_dips=dips,pure_delay_control=raw,guard_prefix_energy_db=prefix_energy))
 print('Dips with common preroll:',dips);print('Pure delay control:',raw);print('Guard prefix maximum energy dB',max(prefix_energy.values()))
if __name__=='__main__':main()
