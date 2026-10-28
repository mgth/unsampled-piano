"""Separate, fixed binaural low-band contrast against neighboring keys.
Freeze source-only eligibility before model comparisons; never change 459 rows.
"""
from support import *
from scipy.signal import resample_poly
spec=importlib.util.spec_from_file_location('player_diagnostic',P/'diagnose.py');diag=importlib.util.module_from_spec(spec);spec.loader.exec_module(diag)
spec=importlib.util.spec_from_file_location('player_validation',P/'validate.py');val=importlib.util.module_from_spec(spec);spec.loader.exec_module(val)

def main():
 sources=json.load(open(R/'bench/references.json'))['sources'];reference={};rows=[];excluded=[]
 for source in ['plath','bivib_grand']:
  reference[source]={}
  for v in ([100] if source=='plath' else [45,56,100]):
   for n in [105,106,107,108]:
    path=c.b.find(sources[source]['dir'],n,{45:4,56:8,100:13}[v]);x=c.b.load(path,stereo=True);on=c.b.onset(x);end=max(0,on-240);band=prev.bands(x)['1-2k'];noise=10*np.log10(diag.power(x,0,end)+1e-30) if end>=960 else None;eligible=noise is not None and band-noise>=12
    reference[source][f'{n}:{v}']=dict(path=str(path),sha256=hashlib.sha256(Path(path).read_bytes()).hexdigest(),onset=on,band=band,noise=noise,snr=band-noise if noise is not None else None,eligible=eligible)
   key=f'108:{v}';anchor=[f'{n}:{v}' for n in [105,106,107] if reference[source][f'{n}:{v}']['eligible']]
   if reference[source][key]['eligible'] and len(anchor)>=2:rows.append(dict(source=source,velocity=v,note=108,anchor=anchor,target=reference[source][key]['band']-np.median([reference[source][a]['band'] for a in anchor])))
   else:excluded.append(dict(source=source,velocity=v,reason='C8 band lacks a 20 ms pre-onset noise window / 12 dB SNR, or fewer than two eligible neighbors',anchor=anchor))
 c.save(P/'binaural-band-references.json',dict(reference=reference,rows=rows,excluded=excluded,policy='Auxiliary corpus: 1–2 kHz C8 relative to median same-band neighbors 105–107. Reference-only SNR>=12 dB and >=20 ms pre-onset window ending 5 ms before detected onset; >=2 neighbors. No gain fit. No changes to historical partial support.'))
 kernel('player_actual_dry');dry={};candidates=json.load(open(P/'close-candidates.json'))
 for n in [105,106,107,108]:dry[f'baseline:{n}']=capture(CONTACT.get(str(n),prev.entry(n)),f'baseline-n{n}','player_actual_dry',hold=1.1,tail=.1,root=O/'binaural',note=n,keep=False)
 for name,e in candidates.items():dry[f'{name}:108']=capture(e,name,'player_actual_dry',hold=1.1,tail=.1,root=O/'binaural',keep=False)
 out=[]
 for r in rows:
  geom='player_actual_dry' if r['source']=='plath' else 'player_actual';x=deepcopy(r);x['model_setup']=geom;x['values']={}
  for name in val.NAMES:
   band=lambda n,v:dry[f'{name if n==108 else "baseline"}:{n}'][str(v)]['bands']['1-2k'] if geom=='player_actual_dry' else prev.measure(val.file('attack','player_actual',name,n,v),n)['bands']['1-2k']
   x['values'][name]=band(108,r['velocity'])-np.median([band(*map(int,a.split(':'))) for a in r['anchor']])
  x['errors']={name:value-x['target'] for name,value in x['values'].items()};out.append(x)
 c.save(P/'binaural-band-comparison.json',dict(rows=out,excluded=excluded,policy='Same reference identities and neighbors for all candidates. Plath compared dry; BiVib compared with fixed studio room (room mismatch remains). Model has bare omnis, no dummy-head/HRTF. No claim of fully matched geometry.'))
 print(json.dumps(c.clean(dict(rows=out,excluded=excluded)),indent=1))
if __name__=='__main__':main()
