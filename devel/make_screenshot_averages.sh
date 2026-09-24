#!/usr/bin/env bash

ple_root="$(git rev-parse --show-toplevel)"
base="${ple_root}/docs/screenshots"
output="${ple_root}/test-results/screenshot-averages"

mkdir -p "${output}"
find "${output}" -maxdepth 1 -type f -name '*-avg*.png' -delete

for folder in $(find "${base}" -type d)
do
  count=$(find "${folder}" -maxdepth 1 -type f -name '*.png' | wc -l | tr -d ' ')

  (( count == 0 )) && continue

  rel="${folder#${base}/}"
  name="$(echo "${rel}" | gsed 's|/|-|g')"

  outimg="${output}/${name}-avg.png"
  cropimg="${output}/${name}-avg-crop.png"

  echo "averaging ${count} images -> ${outimg}"

  magick "${folder}"/*.png -evaluate-sequence mean "${outimg}" &&
    magick "${outimg}" -crop 0x120+0+0 +repage "${cropimg}"
done

echo ""
find "${output}" -maxdepth 1 -type f -name '*-avg*.png' -print | sort

echo ""
echo "open ${output}/*crop.png"
