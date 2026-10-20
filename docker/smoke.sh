#!/usr/bin/env bash
set -euo pipefail

image=${1:?Usage: docker/smoke.sh IMAGE}
smoke_dir=$(mktemp -d)
container=pxna-smoke-$$
volume=pxna-smoke-state-$$
cleanup() {
  status=$?
  if [[ $status -ne 0 ]]; then docker logs "$container" >&2 || true; fi
  docker rm -f "$container" >/dev/null 2>&1 || true
  docker volume rm "$volume" >/dev/null 2>&1 || true
  rm -rf "$smoke_dir"
  exit "$status"
}
trap cleanup EXIT
mkdir "$smoke_dir/library"
chmod 777 "$smoke_dir/library"
printf 'smoke-content' > "$smoke_dir/library/file.txt"
docker run --rm --entrypoint sh "$image" -c 'test "$(id -u)" = 10001; ffmpeg -version >/dev/null; ffprobe -version >/dev/null'
docker run --rm --entrypoint ffmpeg -v "$smoke_dir/library:/fixtures" "$image" \
  -v error -y -f lavfi -i testsrc=duration=4:size=160x90:rate=10 -c:v mpeg4 -pix_fmt yuv420p /fixtures/clip.mp4

docker volume create "$volume" >/dev/null
docker run -d --name "$container" -p 127.0.0.1::6050 -v "$volume:/state" -v "$smoke_dir/library:/libraries" "$image" >/dev/null
refresh_origin() {
  port=$(docker port "$container" 6050/tcp | head -1 | sed 's/.*://')
  origin="http://127.0.0.1:$port"
}
refresh_origin
wait_ready() {
  for attempt in {1..60}; do
    if curl --fail --silent "$origin/api/v1/health" > "$smoke_dir/health.json"; then return; fi
    sleep 1
  done
  return 1
}
wait_ready
curl --fail --silent "$origin/libraries/library" | grep -q '<div id="root">'
curl --fail --silent -X POST -H 'Content-Type: application/json' \
  -d '{"username":"smoke","password":"smoke-password","sessionTransport":"bearer"}' \
  "$origin/api/v1/setup" > "$smoke_dir/setup.json"
token=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["accessToken"])' "$smoke_dir/setup.json")
headers=(--fail --silent -H "Authorization: Bearer $token")
curl "${headers[@]}" -H 'Range: bytes=0-4' -D "$smoke_dir/range.headers" "$origin/api/v1/libraries/library/files/content?path=/file.txt" > "$smoke_dir/range.bin"
[[ $(cat "$smoke_dir/range.bin") == smoke ]]
grep -qi '^content-range: bytes 0-4/13' "$smoke_dir/range.headers"
curl "${headers[@]}" "$origin/api/v1/libraries/library/media-info?path=/clip.mp4" > "$smoke_dir/media.json"
python3 -c 'import json,sys; m=json.load(open(sys.argv[1]))["mediaInfo"]; assert m["kind"] == "video"; assert m["video"]["durationSecs"] == 4 and m["video"]["width"] == 160' "$smoke_dir/media.json"
curl "${headers[@]}" "$origin/api/v1/libraries/library/thumbnails?path=/clip.mp4&width=128&height=128" > "$smoke_dir/thumbnail.bin"
[[ -s "$smoke_dir/thumbnail.bin" ]]
curl "${headers[@]}" -H 'Content-Type: application/json' -d '{"path":"/clip.mp4"}' "$origin/api/v1/libraries/library/video/sessions" > "$smoke_dir/session.json"
playlist=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["session"]["playlistUrl"])' "$smoke_dir/session.json")
curl "${headers[@]}" "$origin$playlist" > "$smoke_dir/playlist.m3u8"
grep -q '^#EXT-X-ENDLIST' "$smoke_dir/playlist.m3u8"
curl "${headers[@]}" "$origin${playlist%/playlist.m3u8}/segments/0.ts" > "$smoke_dir/segment.ts"
[[ -s "$smoke_dir/segment.ts" ]]
curl "${headers[@]}" -X DELETE "$origin${playlist%/playlist.m3u8}" >/dev/null
docker exec "$container" sh -c 'test -f /state/sqlite.db; test -z "$(ls -A /state/video)"'
docker stop -t 10 "$container" >/dev/null
docker start "$container" >/dev/null
refresh_origin
wait_ready
curl "${headers[@]}" "$origin/api/v1/auth/me" > "$smoke_dir/me.json"
python3 -c 'import json,sys; assert json.load(open(sys.argv[1]))["user"]["username"] == "smoke"' "$smoke_dir/me.json"
printf 'Container smoke test passed\n'
