#!/usr/bin/env bash

set -euo pipefail
cd -- "$(dirname -- "${BASH_SOURCE[0]}")"

rm -f *-crop.png

FONT="/Users/vosslab/nsh/SLIDES/djot-slide-builder/assets/fonts/atkinson_hyperlegible_next/AtkinsonHyperlegibleNext-Bold.ttf"

IMAGE_LIST=$(ls *.webp | sort -R | head -n 50)

for img in ${IMAGE_LIST}; do
  [[ "$img" == "stacked-screenshot.webp" ]] && continue
  name="${img%.webp}"
  magick "${img}" \
    -crop 0x250+0+0 +repage \
    -gravity north \
    -background white \
    -fill black \
    -font "${FONT}" \
    -pointsize 12 \
    -splice 0x28 \
    -annotate +8+5 "${name}" \
    "${name}-crop.png"
done

# Stack all cropped images vertically
magick *-crop.png -append stacked-screenshot.webp
rm -f *-crop.png
