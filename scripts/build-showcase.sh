#!/usr/bin/env bash
set -euo pipefail

ROOT=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
FFMPEG=${FFMPEG:-ffmpeg}
OUT=${1:-"$ROOT/branding"}
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

for command in "$FFMPEG"; do
    command -v "$command" >/dev/null 2>&1 || {
        echo "error: ffmpeg is required (override with FFMPEG=/path/to/ffmpeg)" >&2
        exit 1
    }
done

for file in \
    "$ROOT/branding/boot-console.png" \
    "$ROOT/branding/boot-lightdm.png" \
    "$ROOT/branding/lupos-fastfetch.png" \
    "$ROOT/branding/virtualbox_bash.png" \
    "$ROOT/branding/splashart.png"; do
    test -f "$file" || {
        echo "error: missing source capture: $file" >&2
        exit 1
    }
done

mkdir -p "$OUT"

cat >"$TMP/showcase.ass" <<'EOF'
[Script Info]
ScriptType: v4.00+
PlayResX: 1280
PlayResY: 720
WrapStyle: 2

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Title,DejaVu Sans,34,&H00FFFFFF,&H00FFFFFF,&H90000000,&HC0000000,-1,0,0,0,100,100,0,0,3,12,0,7,50,50,22,1
Style: Detail,DejaVu Sans,21,&H00FFD786,&H00FFFFFF,&H90000000,&HC0000000,0,0,0,0,100,100,0,0,3,9,0,7,52,52,68,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:00.00,0:00:04.25,Title,,0,0,0,,FROM RUST ENTRY TO LINUX USERSPACE
Dialogue: 0,0:00:00.00,0:00:04.25,Detail,,0,0,0,,Linux x86 boot protocol  •  bzImage via GRUB  •  systemd
Dialogue: 0,0:00:04.25,0:00:08.50,Title,,0,0,0,,AN UNMODIFIED ARCH USERLAND
Dialogue: 0,0:00:04.25,0:00:08.50,Detail,,0,0,0,,LightDM  •  Xorg  •  XFCE on the Lupos kernel
Dialogue: 0,0:00:08.50,0:00:13.75,Title,,0,0,0,,LINUX BINARIES, UNCHANGED
Dialogue: 0,0:00:08.50,0:00:13.75,Detail,,0,0,0,,bash  •  pacman  •  Xorg  •  XFCE 4.20  •  fastfetch
Dialogue: 0,0:00:13.75,0:00:18.00,Title,,0,0,0,,A LINUX DRIVER-MODULE ABI
Dialogue: 0,0:00:13.75,0:00:18.00,Detail,,0,0,0,,Tested with unchanged Kbuild .ko modules  •  VirtualBox is experimental
Dialogue: 0,0:00:18.00,0:00:22.25,Title,,0,0,0,,PARITY IS MEASURED, NOT ASSUMED
Dialogue: 0,0:00:18.00,0:00:22.25,Detail,,0,0,0,,Linux source tags  •  original Linux tests  •  QEMU compare gates
EOF

cat >"$TMP/boot.ass" <<'EOF'
[Script Info]
ScriptType: v4.00+
PlayResX: 960
PlayResY: 600

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Caption,DejaVu Sans,25,&H00FFFFFF,&H00FFFFFF,&H90000000,&HC0000000,-1,0,0,0,100,100,0,0,3,10,0,7,32,32,20,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
Dialogue: 0,0:00:00.00,0:00:02.00,Caption,,0,0,0,,Lupos boots the Linux userspace ABI
Dialogue: 0,0:00:02.00,0:00:04.00,Caption,,0,0,0,,Unmodified Arch reaches LightDM
Dialogue: 0,0:00:04.00,0:00:07.00,Caption,,0,0,0,,Xorg + XFCE 4.20 on Lupos
EOF

common_inputs=(
    -loop 1 -t 5 -i "$ROOT/branding/boot-console.png"
    -loop 1 -t 5 -i "$ROOT/branding/boot-lightdm.png"
    -loop 1 -t 6 -i "$ROOT/branding/lupos-fastfetch.png"
    -loop 1 -t 5 -i "$ROOT/branding/virtualbox_bash.png"
    -loop 1 -t 5 -i "$ROOT/branding/splashart.png"
)

filter="
[0:v]scale=1280:720:force_original_aspect_ratio=increase,crop=1280:720,
drawbox=x=0:y=0:w=iw:h=112:color=black@0.72:t=fill[s0];
[1:v]scale=1280:720:force_original_aspect_ratio=increase,crop=1280:720,
drawbox=x=0:y=0:w=iw:h=112:color=black@0.72:t=fill[s1];
[2:v]scale=1280:720:force_original_aspect_ratio=increase,crop=1280:720,
drawbox=x=0:y=0:w=iw:h=205:color=black@0.90:t=fill[s2];
[3:v]scale=1280:720:force_original_aspect_ratio=increase,crop=1280:720,
drawbox=x=0:y=0:w=iw:h=112:color=black@0.72:t=fill[s3];
[4:v]scale=1280:720:force_original_aspect_ratio=increase,crop=1280:720,
drawbox=x=0:y=0:w=iw:h=112:color=black@0.72:t=fill[s4];
[s0][s1]xfade=transition=fade:duration=0.75:offset=4.25[x1];
[x1][s2]xfade=transition=fade:duration=0.75:offset=8.5[x2];
[x2][s3]xfade=transition=fade:duration=0.75:offset=13.75[x3];
[x3][s4]xfade=transition=fade:duration=0.75:offset=18.0,
subtitles='$TMP/showcase.ass',format=yuv420p[v]"

"$FFMPEG" -y "${common_inputs[@]}" \
    -filter_complex "$filter" -map "[v]" -an -r 30 \
    -c:v libx264 -preset slow -crf 20 -movflags +faststart \
    "$OUT/lupos-showcase.mp4"

gif_filter="
[0:v]scale=960:600:force_original_aspect_ratio=increase,crop=960:600,
drawbox=x=0:y=0:w=iw:h=72:color=black@0.68:t=fill[s0];
[1:v]scale=960:600:force_original_aspect_ratio=increase,crop=960:600,
drawbox=x=0:y=0:w=iw:h=72:color=black@0.68:t=fill[s1];
[2:v]scale=960:600:force_original_aspect_ratio=increase,crop=960:600,
drawbox=x=0:y=0:w=iw:h=170:color=black@0.90:t=fill[s2];
[s0][s1][s2]concat=n=3:v=1:a=0,
subtitles='$TMP/boot.ass',fps=10,split[p0][p1];
[p0]palettegen=max_colors=160:stats_mode=diff[p];
[p1][p]paletteuse=dither=bayer:bayer_scale=3:diff_mode=rectangle[v]"

"$FFMPEG" -y \
    -loop 1 -t 2 -i "$ROOT/branding/boot-console.png" \
    -loop 1 -t 2 -i "$ROOT/branding/boot-lightdm.png" \
    -loop 1 -t 3 -i "$ROOT/branding/lupos-fastfetch.png" \
    -filter_complex "$gif_filter" -map "[v]" -an -loop 0 \
    "$OUT/lupos-boot.gif"

echo "wrote $OUT/lupos-showcase.mp4"
echo "wrote $OUT/lupos-boot.gif"
