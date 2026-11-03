#!/bin/sh
# shrink.sh: the smaller files made from out/pulse.mp4 (render.mjs), for where
# size counts: the README (GitHub takes videos up to 10 MB) and social posts.
set -e
cd "$(dirname "$0")/out"
make() { # make <name> <size> <fps> <crf>
  ffmpeg -y -loglevel error -i pulse.mp4 -vf "scale=$2:flags=lanczos,fps=$3" \
    -c:v libx264 -preset veryslow -tune animation -crf "$4" -pix_fmt yuv420p -movflags +faststart -an "pulse-$1.mp4"
  echo "pulse-$1.mp4: $(($(wc -c <"pulse-$1.mp4") / 1024)) KB"
}
make 1080p 1920:1080 60 23
make 1080p30 1920:1080 30 23
make 720p 1280:720 30 23
