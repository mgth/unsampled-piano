"""Build the thread's interactive view from the validated measured dataset."""
import keyboard_common as c
import json,sys
from pathlib import Path
payload=json.loads((Path(sys.argv[2]) if len(sys.argv)>2 else c.O/'visual-data.json').read_text())
source=Path(__file__).with_name('keyboard-visual.html').read_text()
assert source.count('__KEYBOARD_DATA__')==1
source=source.replace('__KEYBOARD_DATA__',json.dumps(payload,ensure_ascii=False,separators=(',',':'),allow_nan=False))
assert len(source.encode())<1_000_000
output=Path(sys.argv[1]);output.write_text(source)
print(output)
