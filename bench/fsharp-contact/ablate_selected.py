"""Undo one selected mechanical parameter at a time, after candidate selection."""
from pathlib import Path
import sys,json
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0,str(Path(__file__).parent));import diagnose as d
p=d.R/'out/fsharp-contact/screen/e035/preset.toml';text=p.read_text()
cases={'selected':{},'undo_shank_loss':{'shank_loss':.1},'undo_shank_frequency':{'shank_freq':300},'undo_shank_share':{'shank_share':.3},'undo_contact_scale':{'hardness':.3},'undo_hysteresis':{'hysteresis':.1}}
def one(item):
 name,fields=item;name='selected_'+name;p=d.O/'diagnostics'/f'{name}.toml';p.write_text(d.patch(text,fields));d.c.run([d.BIN,p,d.O/'diagnostics'/name,102,','.join(map(str,d.V)),5]);return name,d.metrics(name,d.V)
with ThreadPoolExecutor(4) as ex:r=dict(ex.map(one,cases.items()))
d.c.save(d.P/'selected-ablations.json',r);d.c.save(d.P/'selected-ablation-design.json',dict(source='e035',cases=cases))
import numpy as np
for name,rows in r.items():
 print(name,'ff',rows['100']['audio']['partials'][:2],'dips',[max(np.maximum.accumulate(a)-a) for a in [[rows[str(v)]['audio']['partials'][k] for v in d.V] for k in [0,1]]],flush=True)
