#!/usr/bin/env bash
#
# Deploy the built site to o2switch over FTP(S).
#
# Usage:  npm run deploy         (builds, then uploads dist/)
#         SKIP_BUILD=1 npm run deploy   (upload the existing dist/ as-is)
#
# Credentials live in a local, gitignored .env.deploy (copy .env.deploy.example).
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ ! -f .env.deploy ]]; then
  echo "error: missing .env.deploy — copy .env.deploy.example and fill in your FTP credentials." >&2
  exit 1
fi

# Parse .env.deploy WITHOUT sourcing it — values are taken literally, so a
# password may contain spaces, $, !, &, #, quotes, etc. without breaking.
while IFS= read -r line || [[ -n "$line" ]]; do
  line="${line%$'\r'}"                       # tolerate CRLF line endings
  [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
  [[ "$line" != *=* ]] && continue
  key="${line%%=*}"
  key="${key//[[:space:]]/}"                  # trim whitespace around the key
  val="${line#*=}"
  [[ "$key" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || continue
  # strip one matching pair of surrounding quotes, if present
  if [[ ( "$val" == \"*\" || "$val" == \'*\' ) && ${#val} -ge 2 ]]; then
    val="${val:1:${#val}-2}"
  fi
  printf -v "$key" '%s' "$val"
  export "$key"
done < .env.deploy

: "${FTP_HOST:?set FTP_HOST in .env.deploy}"
: "${FTP_USER:?set FTP_USER in .env.deploy}"
: "${FTP_PASS:?set FTP_PASS in .env.deploy}"
FTP_REMOTE_DIR="${FTP_REMOTE_DIR:-/}"
FTP_SSL="${FTP_SSL:-true}"
FTP_SSL_VERIFY="${FTP_SSL_VERIFY:-false}"
FTP_DELETE="${FTP_DELETE:-false}"

if ! command -v lftp >/dev/null 2>&1; then
  echo "error: lftp is not installed." >&2
  exit 1
fi

if [[ "${SKIP_BUILD:-}" != "1" ]]; then
  echo "==> Building site"
  npm run build
fi

if [[ ! -d dist ]]; then
  echo "error: dist/ not found — run a build first (or unset SKIP_BUILD)." >&2
  exit 1
fi

# --delete removes remote files no longer present locally. Guarded because it is
# destructive: only enable it when FTP_REMOTE_DIR is a dedicated docroot. The
# host's own entries (.well-known/ for certificates, cgi-bin/, .ftpquota) are
# excluded from the mirror, so --delete never touches them.
DELETE_FLAG=""
if [[ "${FTP_DELETE}" == "true" ]]; then
  DELETE_FLAG="--delete"
fi

echo "==> Uploading dist/ -> ${FTP_HOST}:${FTP_REMOTE_DIR} (delete=${FTP_DELETE})"
lftp -c "
set ftp:ssl-allow ${FTP_SSL};
set ftp:ssl-force ${FTP_SSL};
set ssl:verify-certificate ${FTP_SSL_VERIFY};
set net:max-retries 2;
set net:timeout 20;
open -u '${FTP_USER}','${FTP_PASS}' '${FTP_HOST}';
mirror --reverse ${DELETE_FLAG} --verbose --parallel=4 \
  --exclude-glob .git/ --exclude-glob .DS_Store \
  --exclude-glob .well-known/ --exclude-glob cgi-bin/ --exclude-glob .ftpquota \
  dist/ '${FTP_REMOTE_DIR}';
bye;
"

echo "==> Done. Live at https://unsampled.mgth.fr/"
