"""Strict research-only preset field overrides."""
import re

def patch(text,fields):
 for key,value in fields.items():
  if '.' in key:
   section,key=key.split('.',1);pattern=rf'(?ms)(^\[{re.escape(section)}\]\n)(.*?)(?=^\[|\Z)';match=re.search(pattern,text);assert match
   block,count=re.subn(rf'^{key} = .*$',f'{key} = {str(value).lower()}',match[2],flags=re.M);assert count==1
   text=text[:match.start(2)]+block+text[match.end(2):]
  else:text,count=re.subn(rf'^{key} = .*$',f'{key} = {str(value).lower()}',text,flags=re.M);assert count==1
 return text
