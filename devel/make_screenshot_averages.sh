#!/usr/bin/env bash

ple_root="$(git rev-parse --show-toplevel)"
base="${ple_root}/docs/screenshots"
output="${ple_root}/test-results/screenshot-averages"

mkdir -p "${output}"
find "${output}" -maxdepth 1 -type f -name '*-avg*.png' -delete

shopt -s nullglob
while IFS= read -r -d '' folder
do
  images=("${folder}"/*.png)
  count=${#images[@]}

  (( count == 0 )) && continue

  rel="${folder#${base}/}"
  name="${rel//\//-}"

  outimg="${output}/${name}-avg.png"
  cropimg="${output}/${name}-avg-crop.png"

  echo "averaging ${count} images -> ${outimg}"

  magick "${images[@]}" -evaluate-sequence mean "${outimg}" &&
    magick "${outimg}" -crop 0x120+0+0 +repage "${cropimg}"
done < <(find "${base}" -type d -print0)

echo ""
find "${output}" -maxdepth 1 -type f -name '*-avg*.png' -print | sort

echo ""
echo "open ${output}/*crop.png"
