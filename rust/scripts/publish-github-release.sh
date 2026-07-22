#!/usr/bin/env bash
# Push current main to GitHub and publish a release with dist APKs.
# Usage:
#   ./scripts/publish-github-release.sh [versionName]
#   (versionName defaults to workspace Cargo.toml version)
#
# Requires git credentials that can push + create releases (HTTPS token with repo scope).
# Set FIG_SKIP_PUSH=1 to only create the release without git push.
# Set FIG_SKIP_RELEASE=1 to only push without creating a release.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "$ROOT/.." && pwd)"
cd "$REPO_ROOT"

VERSION_NAME="${1:-}"
if [[ -z "$VERSION_NAME" ]]; then
  VERSION_NAME="$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed -E 's/.*"([^"]+)".*/\1/')"
fi
TAG="v${VERSION_NAME}"

echo "=== Publish GitHub: $TAG ==="

# Resolve token from git credential helper (never print it)
TOKEN="$(
  python3 - <<'PY'
import subprocess
try:
    out = subprocess.check_output(
        ["git", "credential", "fill"],
        input="protocol=https\nhost=github.com\n\n",
        text=True,
        timeout=8,
    )
except Exception:
    raise SystemExit(0)
for line in out.splitlines():
    if line.startswith("password="):
        print(line.split("=", 1)[1], end="")
        break
PY
)"

if [[ -z "${TOKEN}" ]]; then
  echo "No GitHub token from git credential helper." >&2
  echo "Push manually: git push origin main" >&2
  echo "Then create a release at https://github.com/linuxbased79/FixItGarage/releases/new" >&2
  exit 1
fi

API="https://api.github.com/repos/linuxbased79/FixItGarage"
AUTH=(-H "Authorization: Bearer ${TOKEN}" -H "Accept: application/vnd.github+json" -H "X-GitHub-Api-Version: 2022-11-28")

if [[ "${FIG_SKIP_PUSH:-0}" != "1" ]]; then
  # Commit any staged? No — only push what's already committed.
  if ! git diff --quiet || ! git diff --cached --quiet; then
    echo "Warning: uncommitted local changes exist; only committed history will push." >&2
  fi
  BRANCH="$(git rev-parse --abbrev-ref HEAD)"
  echo "Pushing ${BRANCH} → origin..."
  git push origin "$BRANCH"
else
  echo "Skipping git push (FIG_SKIP_PUSH=1)"
fi

if [[ "${FIG_SKIP_RELEASE:-0}" == "1" ]]; then
  echo "Skipping GitHub release (FIG_SKIP_RELEASE=1)"
  exit 0
fi

# Notes from CHANGELOG section for this version, else a short default
NOTES="$(
  python3 - <<PY
from pathlib import Path
ver = "${VERSION_NAME}"
text = Path("CHANGELOG.md").read_text(encoding="utf-8", errors="replace")
lines = text.splitlines()
start = None
for i, line in enumerate(lines):
    if line.strip() == f"## {ver}":
        start = i
        break
if start is None:
    print(f"FixItGarage {ver}\\n\\nSee CHANGELOG.md and commit history.")
else:
    body = []
    for line in lines[start + 1 :]:
        if line.startswith("## "):
            break
        body.append(line)
    print("\\n".join(body).strip() or f"FixItGarage {ver}")
PY
)"

# Create release if missing
EXISTING="$(curl -sS "${AUTH[@]}" "${API}/releases/tags/${TAG}" || true)"
RELEASE_ID="$(python3 -c "import json,sys; d=json.loads(sys.argv[1]); print(d.get('id') or '')" "$EXISTING" 2>/dev/null || true)"

if [[ -z "$RELEASE_ID" ]]; then
  echo "Creating release ${TAG}..."
  PAYLOAD="$(NOTES="$NOTES" VERSION_NAME="$VERSION_NAME" TAG="$TAG" python3 - <<'PY'
import json, os
print(json.dumps({
    "tag_name": os.environ["TAG"],
    "name": f"FixItGarage {os.environ['VERSION_NAME']}",
    "body": os.environ["NOTES"],
    "draft": False,
    "prerelease": False,
    "target_commitish": "main",
}))
PY
)"
  CREATED="$(curl -sS -X POST "${AUTH[@]}" "${API}/releases" -d "$PAYLOAD")"
  RELEASE_ID="$(python3 -c "
import json, sys
d = json.loads(sys.argv[1])
rid = d.get('id')
if not rid:
    sys.stderr.write(sys.argv[1][:500] + '\n')
    sys.exit(1)
print(rid)
" "$CREATED")"
  echo "Release id=$RELEASE_ID"
else
  echo "Release ${TAG} already exists (id=$RELEASE_ID)"
fi

# Upload APKs from rust/dist
shopt -s nullglob
ASSETS=("$ROOT/dist/FixItGarage-${VERSION_NAME}"-*.apk)
# Prefer packaged (not -raw)
UPLOAD_LIST=()
for f in "${ASSETS[@]}"; do
  case "$f" in
    *-raw.apk|*.idsig) continue ;;
    *.apk) UPLOAD_LIST+=("$f") ;;
  esac
done

if [[ ${#UPLOAD_LIST[@]} -eq 0 ]]; then
  echo "No APKs found in $ROOT/dist for ${VERSION_NAME}" >&2
  echo "Build first: ./scripts/release-apks.sh ${VERSION_NAME}" >&2
  exit 1
fi

# Existing asset names (to replace or skip)
EXISTING_ASSETS="$(curl -sS "${AUTH[@]}" "${API}/releases/${RELEASE_ID}/assets")"

for APK in "${UPLOAD_LIST[@]}"; do
  NAME="$(basename "$APK")"
  # Delete existing asset with same name (re-publish)
  AID="$(NAME="$NAME" python3 -c "
import json,os,sys
assets=json.loads(sys.stdin.read() or '[]')
name=os.environ['NAME']
for a in assets:
    if a.get('name')==name:
        print(a['id']); break
" <<<"$EXISTING_ASSETS" 2>/dev/null || true)"
  if [[ -n "${AID:-}" ]]; then
    echo "Replacing existing asset $NAME (id=$AID)..."
    curl -sS -X DELETE "${AUTH[@]}" "${API}/releases/assets/${AID}" >/dev/null || true
  fi
  echo "Uploading $NAME ($(du -h "$APK" | cut -f1))..."
  UP="$(curl -sS -X POST \
    "${AUTH[@]}" \
    -H "Content-Type: application/vnd.android.package-archive" \
    --data-binary @"$APK" \
    "https://uploads.github.com/repos/linuxbased79/FixItGarage/releases/${RELEASE_ID}/assets?name=${NAME}")"
  python3 -c "import json,sys; d=json.loads(sys.argv[1]);
print('  →', d.get('browser_download_url') or d.get('message','?'))" "$UP"
done

echo
echo "Done: https://github.com/linuxbased79/FixItGarage/releases/tag/${TAG}"
