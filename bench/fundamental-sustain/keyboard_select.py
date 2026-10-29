"""Select improvements against fixed reference coverage and timbre guards."""
import keyboard_common as c
import keyboard_diagnose as d
import json,math
import numpy as np
ASSEMBLY_REJECTIONS={80:{'h0.7_f0.7':'H2 +3.205 dB in the assembled 88-key validation'}}

def error(n,rows,refs,base):
 errors=[];means=[]
 for v in c.VELOCITIES:
  k=f'{n}|{v}'
  if d.assess(n,base[k],refs[k])['status']=='insufficient':continue
  a=d.assess(n,rows[k],refs[k])
  if a['status']=='insufficient':return float('inf')
  errors.extend(a['excess']);means.extend(a['mean_delta'])
 return float(np.mean(np.square(errors))+.025*np.mean(np.square(means))) if errors else float('inf')

def guards(n,rows,refs,base):
 attack=[];upper=[]
 for v in c.VELOCITIES:
  k=f'{n}|{v}';a=np.array(base[k]['levels'],float);b=np.array(rows[k]['levels'],float)
  if not np.isfinite(b[[0,3,6],0]).all():return 'H1 incomplet'
  if not np.isfinite(a[[0,3,6],0]).all():return 'H1 initial incomplet'
  attack.append(abs(b[0,0]-a[0,0]))
  # Only audible upper partials; same members in both conditions.
  mask=np.isfinite(a[7,1:]) & (a[7,1:]>=np.nanmax(a[0])-45)
  if not np.isfinite(b[7,1:][mask]).all():return 'H2–H8 deviennent illisibles'
  upper.extend(abs(b[7,1:][mask]-a[7,1:][mask]))
  previous=d.assess(n,base[k],refs[k]);candidate=d.assess(n,rows[k],refs[k])
  if previous['status']!='insufficient':
   if candidate['status']=='insufficient':return 'couverture H1 perdue'
   for i in range(2):
    if previous['directions'][i]=='within':
     if candidate['directions'][i]!='within':return 'nouvelle alerte'
     if abs(candidate['value'][i]-previous['value'][i])>3:return 'zone correcte modifiée >3 dB'
 if max(attack)>1:return 'attaque H1 >1 dB'
 if upper and max(upper)>3:return 'H2–H8 >3 dB'
 return None

def main():
 search=json.loads((c.O/'search.json').read_text());refs=json.loads((c.O/'references.json').read_text())['notes'];base=json.loads((c.O/'current.json').read_text())
 selected={};outcomes={}
 for n in search['notes']:
  before=error(n,base,refs,base);candidates=[];rejects={}
  for name,rows in search['results'].items():
   if name in ASSEMBLY_REJECTIONS.get(n,{}):
    rejects[name]=ASSEMBLY_REJECTIONS[n][name];continue
   reason=guards(n,rows,refs,base)
   if reason:rejects[name]=reason;continue
   after=error(n,rows,refs,base)
   if after<before*.7:candidates.append((after,name))
   else:rejects[name]='gain inférieur à 30 %'
  outcomes[n]=dict(before_error=before,rejected=rejects)
  if candidates:
   after,name=min(candidates);h,f=map(float,name[1:].split('_f'))
   selected[n]={'polarization.h_coupling':h,'polarization.cross':math.sqrt(h)*f}
   outcomes[n].update(variant=name,after_error=after,reduction=1-after/before)
  print(n,outcomes[n].get('variant','non retenu'),outcomes[n].get('reduction',''),flush=True)
 c.save(c.O/'selection.json',dict(selected=selected,outcomes=outcomes))
 path=c.bundle(c.O/'prototype_keyboard',selected)
 print('selected:',len(selected),path,flush=True)

if __name__=='__main__':main()
