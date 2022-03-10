#!/bin/bash
filename=$1
outputfolder=$2

base=$(basename $filename)
frame_id="${base%.*}"
echo "Converting $filename..."
convert -size 1920x1080 -depth 8 -define webp:lossless=true "$filename" "$outputfolder/$frame_id.webp"
