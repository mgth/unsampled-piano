// French texts of the parameters (src/data/params.fr.json) against the
// English ones exported from the code (src/data/params.json): lists what is
// missing, and what was translated from an English text that has changed
// since (each help entry keeps a short hash of its source in `src`).
//   node scripts/check-fr.mjs           report; exit 1 if anything is off
//   node scripts/check-fr.mjs --stamp   record the current sources' hashes
import fs from 'node:fs';
import crypto from 'node:crypto';

const EN = JSON.parse(fs.readFileSync('src/data/params.json', 'utf8'));
const FILE = 'src/data/params.fr.json';
const fr = JSON.parse(fs.readFileSync(FILE, 'utf8'));
const strip = (p) => p.replace(/\[\d+\]/g, '');
const hash = (s) => crypto.createHash('sha1').update(s).digest('hex').slice(0, 10);

const help = new Map();
for (const p of EN.params) help.set(strip(p.path), `${p.what}\n${p.how}`);
const groups = new Set(EN.params.map((p) => (strip(p.path).includes('.') ? strip(p.path).replace(/\.[^.]+$/, '') : 'general')));

const problems = [];
for (const p of EN.params) if (!fr.labels[p.path]) problems.push(`label ${p.path}`);
for (const [k, src] of help) {
  const t = fr.help[k];
  if (!t?.what || !t?.how) problems.push(`help ${k}`);
  else if (process.argv.includes('--stamp')) t.src = hash(src);
  else if (t.src !== hash(src)) problems.push(`stale help ${k} (English text changed)`);
}
for (const page of EN.pages) {
  if (!fr.pages[page.title]) problems.push(`page ${page.title}`);
  for (const c of page.curves) if (!fr.curves[c.title]) problems.push(`curve ${c.title}`);
}
for (const g of groups) if (!fr.groups[g]) problems.push(`group ${g}`);

if (process.argv.includes('--stamp')) fs.writeFileSync(FILE, JSON.stringify(fr, null, 2) + '\n');
console.log(problems.length ? problems.join('\n') : 'French parameter texts: complete and current.');
process.exit(problems.length && !process.argv.includes('--stamp') ? 1 : 0);
