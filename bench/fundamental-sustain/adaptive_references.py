"""Measured references plus labelled, within-piano neighbour interpolation.

Never borrow velocity layers, count two neighbours as two pianos, extrapolate
at the keyboard edge, or interpolate from an already estimated reference.
A max six-semitone bracket allows sparse every-third-key sample collections.
"""
import json
import numpy as np
import keyboard_common as c
from adaptive_scan import O,FOCUS
MODEL_NAMES={'main','current','previous','candidate'}
MAX_SPAN=6

def valid(row):return row is not None and bool(row['curve']) and all(z is not None and np.isfinite(z) for z in row['curve'])

def compatible(row,n):
 expected=[.25,.5,.75,1.1] if n<40 else [.03,.25,.5,.75,1.1]
 return (valid(row) and len(row['curve'])==len(expected)
         and row.get('times',expected)==expected and row.get('anchor',expected[0])==expected[0])

def bracket(rows,n,v,piano):
 lower=[];upper=[]
 for m in range(max(21,n-MAX_SPAN),min(108,n+MAX_SPAN)+1):
  if m==n or (m<40)!=(n<40):continue
  r=rows.get(f'{m}|{v}',{}).get(piano)
  if compatible(r,n):(lower if m<n else upper).append(m)
 if not lower or not upper:return None
 lo=max(lower);hi=min(upper)
 if hi-lo>MAX_SPAN:return None
 a=np.array(rows[f'{lo}|{v}'][piano]['curve']);b=np.array(rows[f'{hi}|{v}'][piano]['curve'])
 w=(n-lo)/(hi-lo);return dict(neighbours=[lo,hi],weights=[1-w,w],curve=((1-w)*a+w*b).tolist(),lo=np.minimum(a,b).tolist(),hi=np.maximum(a,b).tolist())

def calibration(rows):
 errors=[];records=[]
 for k,rs in rows.items():
  n,v=map(int,k.split('|'))
  for nm,r in rs.items():
   if nm in MODEL_NAMES or not valid(r):continue
   estimate=bracket(rows,n,v,nm)
   if estimate:
    delta=abs(np.array(estimate['curve'])-r['curve']);errors.extend(delta[1:].tolist())
    records.append(dict(key=k,piano=nm,neighbours=estimate['neighbours'],errors=delta.tolist()))
 # Calibrated uncertainty with a 3 dB floor; not a statistical confidence interval.
 pad=max(3.,float(np.quantile(errors,.9))) if errors else 6.
 by_register={}
 for register,test in [('bass',lambda n:n<40),('treble',lambda n:n>=40)]:
  e=[v for r in records if test(int(r['key'].split('|')[0])) for v in r['errors'][1:]]
  by_register[register]=max(3.,float(np.quantile(e,.9))) if e else 6.
 return dict(pad_db=pad,pad_by_register=by_register,held_out_records=records,method='90th percentile absolute leave-one-key-out error within each register/anchor method; minimum 3 dB')

def make_reference(rows,n,v,pad):
 actual={nm:r['curve'] for nm,r in rows[f'{n}|{v}'].items() if nm not in MODEL_NAMES and valid(r)}
 estimates={}
 if len(actual)<2:
  sources=set(nm for rs in rows.values() for nm in rs if nm not in MODEL_NAMES)
  for nm in sorted(sources-actual.keys()):
   z=bracket(rows,n,v,nm)
   if z:
    z['lo']=[0]+[p-pad for p in z['lo'][1:]];z['hi']=[0]+[p+pad for p in z['hi'][1:]]
    estimates[nm]=z
 return dict(actual=actual,estimated=estimates,count=len(actual)+len(estimates))

def assess(row,reference):
 a=np.array([np.nan if z is None else z for z in row['curve']]);target=list(reference['actual'].values())+[z['curve'] for z in reference['estimated'].values()]
 result=dict(reference_count=reference['count'],measured_count=len(reference['actual']),estimated_count=len(reference['estimated']))
 if not np.isfinite(a).all() or len(target)<2:return result|dict(status='insufficient',excess=[],mean_delta=[])
 lo=list(reference['actual'].values())+[z['lo'] for z in reference['estimated'].values()]
 hi=list(reference['actual'].values())+[z['hi'] for z in reference['estimated'].values()]
 lo=np.min(lo,axis=0);hi=np.max(hi,axis=0);mean=np.mean(target,axis=0)
 # Compare 0.5 and 1.1 s; all four observations must remain measurable.
 ix=[row['times'].index(t) for t in [.5,1.1]];value=a[ix];excess=np.maximum(value-hi[ix],0)+np.minimum(value-lo[ix],0)
 directions=['fast' if z<-3 else 'slow' if z>3 else 'within' for z in excess]
 status='mixed' if 'fast' in directions and 'slow' in directions else 'fast' if 'fast' in directions else 'slow' if 'slow' in directions else 'within'
 return result|dict(status=status,directions=directions,excess=excess.tolist(),mean_delta=(value-mean[ix]).tolist(),mean=mean.tolist(),lo=lo.tolist(),hi=hi.tolist(),value=value.tolist())

def main():
 rows=json.loads((O/'measurements.json').read_text())['rows'];cal=calibration(rows)
 references={f'{n}|{v}':make_reference(rows,n,v,cal['pad_by_register']['bass' if n<40 else 'treble']) for n in FOCUS for v in c.VELOCITIES}
 diagnoses={k:assess(rows[k]['previous'],r) for k,r in references.items()}
 c.save(O/'references.json',dict(calibration=cal,references=references,diagnoses=diagnoses))
 print('cross-validation',len(cal['held_out_records']),'windows',len(cal['held_out_records'])*3,'pad',round(cal['pad_db'],2))
 for k,r in references.items():print(k,diagnoses[k]['status'],'direct',list(r['actual']),'estimated',{nm:z['neighbours'] for nm,z in r['estimated'].items()})

if __name__=='__main__':main()
