"""Additional diagnostic: detune and the inactive fallback bridge law.
bridge.g_1k is not used when a modal soundboard is present; its null result
must not be interpreted as evidence that physical bridge coupling is irrelevant.
"""
from pathlib import Path
import sys
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0,str(Path(__file__).parent));import diagnose as d
CASES={'no_detune':{'unison.width_mid':0,'unison.width_treble':0},'weak_bridge':{'bridge.g_1k':.000046},'strong_bridge':{'bridge.g_1k':.0046}}
def one(item):
 name,fields=item;p=d.O/'diagnostics'/f'{name}.toml';p.write_text(d.patch(d.BASE,fields))
 d.c.run([d.BIN,p,d.O/'diagnostics'/name,102,','.join(map(str,d.V)),5]);return name,d.metrics(name,d.V)
if __name__=='__main__':
 d.c.save(d.P/'unison-design.json',dict(cases=CASES,velocities=d.V,bridge_law_inactive=True))
 with ThreadPoolExecutor(3) as ex:results=dict(ex.map(one,CASES.items()))
 d.c.save(d.P/'unison-diagnostics.json',results)
