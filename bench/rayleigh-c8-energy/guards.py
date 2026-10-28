"""Dense energy and within-piano neighboring-key level guards."""
from support import *
import importlib.util
spec=importlib.util.spec_from_file_location('c8_validation',P/'validate.py');val=importlib.util.module_from_spec(spec);spec.loader.exec_module(val)

def main():
 curves={};dense_summary={}
 for name in val.NAMES:
  curves[name]={str(v):prev.measure(val.file('dense',name,108,v),108) for v in range(28,128)}
 for name in ['recovered','recovered_mf']:
  dense_summary[name]={}
  for baseline in ['balanced','contact']:
   delta={k:[curves[name][str(v)]['bands']['1-2k']-curves[baseline][str(v)]['bands']['1-2k'] if k=='1-2k' else curves[name][str(v)][k]-curves[baseline][str(v)][k] for v in range(28,128)] for k in ['1-2k','p1','level_preroll']}
   dense_summary[name][baseline]={k:dict(min=min(y),max=max(y),min_velocity=28+int(np.argmin(y)),max_velocity=28+int(np.argmax(y))) for k,y in delta.items()}
 old=json.load(open(prev.P/'level-evenness.json'));rows=[]
 for r in old['rows']:
  if r['note']!=108:continue
  x={k:r[k] for k in ['source','note','velocity','anchor','target']};x['values']={name:r['values'][name] for name in ['balanced','contact']};v=r['velocity']
  for name in ['recovered','recovered_mf']:
   x['values'][name]=r['values']['contact']+prev.measure(val.file('attack',name,108,v),108)['level_preroll']-BASELINES['contact'][str(v)]['level_preroll']
  rows.append(x)
 summary={v:{name:dict(bias=np.mean([r['values'][name]-r['target'] for r in rows if r['velocity']==v]),mae=np.mean([abs(r['values'][name]-r['target']) for r in rows if r['velocity']==v]),rows=sum(r['velocity']==v for r in rows)) for name in val.NAMES} for v in [45,56,100]}
 c.save(P/'energy-guards.json',dict(dense_summary=dense_summary,dense_curves=curves,evenness_rows=rows,evenness_summary=summary,policy='Same 100 velocities and same within-piano neighbor/source identities. No gain fitted. Within-source contrasts exploratory: 2/1/5 pianos at pp/mf/forte.'))
 print(json.dumps(c.clean(dict(dense=dense_summary,evenness=summary)),indent=1))
if __name__=='__main__':main()
