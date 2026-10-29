from study import *

def held(name,stage):
 p=PAR['102'];k=np.arange(1,5);f=p['f0']*k*np.sqrt(1+p['B']*k*k)
 return {f'102:{v}':c.attack(b.load(str(O/name/stage/f'n102v{v}.wav'),stereo=True),f,.5,.5) for v in VEL}

def main():
 base=held('control','screen');result={}
 for stage in ['screen','refine','factorial']:
  for name in json.load(open(P/f'{stage}-variants.json')):
   m=held(name,stage)
   result[name]=dict(metrics=m,scores={str(v):scores(m,v,'sustain') for v in VEL},delta_p2={str(v):m[f'102:{v}']['partials'][0]-base[f'102:{v}']['partials'][0] for v in VEL})
 save(P/'sustain-screen.json',result)
 for name in json.load(open(P/'dense-summary.json'))['passed']:print(name,result[name]['delta_p2'])
if __name__=='__main__':main()
