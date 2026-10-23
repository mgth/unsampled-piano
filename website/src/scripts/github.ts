// GitHub API fetch with a short sessionStorage cache. Unauthenticated calls are
// rate-limited to 60/hour per IP, and several parts of the site hit the API on
// load (download cards, stars badge) — caching keeps repeat page views within a
// session from burning through the quota.
const TTL_MS = 10 * 60 * 1000;

export async function fetchGitHubJson(path: string): Promise<any> {
  const key = `gh:${path}`;
  try {
    const hit = sessionStorage.getItem(key);
    if (hit) {
      const { t, data } = JSON.parse(hit);
      if (Date.now() - t < TTL_MS) return data;
    }
  } catch {
    // Storage unavailable (private mode) or corrupt entry — fall through.
  }
  const res = await fetch(`https://api.github.com${path}`, {
    headers: { Accept: 'application/vnd.github+json' },
  });
  if (!res.ok) throw new Error('http ' + res.status);
  const data = await res.json();
  try {
    sessionStorage.setItem(key, JSON.stringify({ t: Date.now(), data }));
  } catch {
    // Quota exceeded or storage unavailable — serve uncached.
  }
  return data;
}
