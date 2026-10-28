"""Reject any new late-velocity dip hidden by the existing global maximum."""
from guard_search import *
def main():
 wet={v:tuple(wavfile.read(O/'dense-screen'/kind/f'n102v{v}.wav')[1].astype(float) for kind in ['control','driven']) for v in range(28,128)}
 par=PAR['102'];k=np.arange(1,5);freq=par['f0']*k*np.sqrt(1+par['B']*k*k);control={v:c.attack(d,freq) for v,(d,n) in wet.items()};rows=[]
 for maximum in [.05,.1,.12,.13,.14,.15,.16,.18,.2,.25]:
  for end in [96,100,104,112,120,127]:
   m={v:c.attack(guarded(d,n,v,maximum,end,start=88,shape='linear'),freq) for v,(d,n) in wet.items()}
   dips=[float(max(np.maximum.accumulate([m[v]['partials'][i] for v in wet])-[m[v]['partials'][i] for v in wet])) for i in [0,1]]
   late=np.array([m[v]['partials'][1] for v in range(88,128)]);late_dip=float(max(np.maximum.accumulate(late)-late))
   delta={str(v):[m[v]['partials'][i]-control[v]['partials'][i] for i in [0,1]] for v in [45,56,100]}
   rows.append(dict(maximum=maximum,start=88,end=end,shape='linear',dips=dips,late_dip=late_dip,delta=delta))
 c.save(P/'guard-monotone-screen.json',rows)
 passing=[r for r in rows if r['dips'][0]<=1.29184 and r['dips'][1]<=1.39638 and r['late_dip']<=.01 and abs(r['delta']['100'][0])<.01]
 assert passing;best=min(passing,key=lambda r:abs(8.4+r['delta']['100'][1]))
 c.save(P/'guard-selected.json',dict(**best,note=102,unchanged_below_or_at=88,cutoff_hz=7500,taps=129,latency_samples=64,classification='Targeted velocity-dependent voicing crossfade; not a physical radiation law.',selected_on='Dense model guard and previous mean forte excess. Same calibration notes, no held-out test.',criteria='P2/P3 maximum dips grow <0.01 dB; no new late drawdown >0.01 dB over 88..127; P2 forte changes <0.01 dB; all other notes and v<=88 bypass.'))
 print('SELECTED',best,flush=True)
if __name__=='__main__':main()
