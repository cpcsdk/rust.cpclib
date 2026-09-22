#!/usr/bin/env bash
# Real-world smoke test for img2cpc's true-color conversion pipeline
# (--dither/--resize-filter/--colors): converts a handful of the images
# classically used in image-processing research (Cameraman, the
# "Astronaut" portrait - scikit-image's licensing-safe stand-in for Lena,
# a cat portrait, a full-hue colorwheel, a coffee cup) to Amstrad CPC
# screens, then renders the actual generated .scr files back to PNG with
# cpc2img so the result can be inspected visually.
#
# Every step uses only what ships with this repo (img2cpc, cpc2img) plus
# scikit-image, which bundles these exact images as redistributable test
# data (BSD-licensed) - no images are fetched from the network.
#
# Usage:
#   cargo build -p cpclib-imgconverter --bin img2cpc --bin cpc2img
#   python3 -m pip install --user scikit-image pillow   # one-time
#   cpclib-imgconverter/examples/true_color_research_images.sh
#
# Output goes to target/examples/true_color_research/ - a PNG per source
# image, converted at 320x200 (mode 0/1 standard screen), viewable directly.

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
bin_dir="$repo_root/target/debug"
out_dir="$repo_root/target/examples/true_color_research"
img_dir="$out_dir/source_images"

img2cpc="$bin_dir/img2cpc"
cpc2img="$bin_dir/cpc2img"

for tool in "$img2cpc" "$cpc2img"; do
    if [[ ! -x "$tool" ]]; then
        echo "error: $tool not found - build it first:" >&2
        echo "  cargo build -p cpclib-imgconverter --bin img2cpc --bin cpc2img" >&2
        exit 1
    fi
done

if ! python3 -c "import skimage" >/dev/null 2>&1; then
    echo "error: scikit-image is required to source the test images:" >&2
    echo "  python3 -m pip install --user scikit-image pillow" >&2
    exit 1
fi

mkdir -p "$img_dir" "$out_dir"

echo "Exporting scikit-image's bundled research images..."
python3 - "$img_dir" <<'PY'
import sys
from skimage import data
from PIL import Image

img_dir = sys.argv[1]

def save(name, arr):
    img = Image.fromarray(arr)
    if img.mode == "L":
        img = img.convert("RGB")
    img.save(f"{img_dir}/{name}.png")

save("camera", data.camera())        # the classic "Cameraman" test image
save("astronaut", data.astronaut())  # skimage's Lena replacement
save("chelsea", data.chelsea())      # cat portrait
save("colorwheel", data.colorwheel())
save("coffee", data.coffee())
PY

# name mode dither
conversions=(
    "camera 0 floyd-steinberg"
    "astronaut 1 ordered"
    "chelsea 0 atkinson"
    "colorwheel 1 floyd-steinberg"
    "coffee 0 stucki"
)

for entry in "${conversions[@]}"; do
    read -r name mode dither <<<"$entry"
    src="$img_dir/$name.png"
    scr="$out_dir/$name.scr"
    pal="$out_dir/$name.pal"
    png="$out_dir/${name}_cpc.png"

    echo "Converting $name (mode $mode, $dither)..."
    "$img2cpc" --mode "$mode" --dither "$dither" "$src" scr -o "$scr" -p "$pal"

    # Mode 0 packs 2 pixels/byte, so its pixels are twice as wide as mode
    # 1/2's and need --mode0ratio to display at the right aspect ratio; mode
    # 1/2 are already square-ish and must NOT be doubled again.
    ratio_flag=()
    if [[ "$mode" == "0" ]]; then
        ratio_flag=(--mode0ratio)
    fi
    "$cpc2img" "$scr" "$png" -m "$mode" --ga-pal "$pal" "${ratio_flag[@]}" screen --width 80
done

echo
echo "Done. Rendered screens are in $out_dir/*_cpc.png"
