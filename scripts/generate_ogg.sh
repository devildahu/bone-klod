#!/bin/bash
set -e
for file in chill introChill introTheremin orchestralFinale orchestral theremin ; do
  mpg123 -w "target/convert.wav" "target/raw_files/$file.mp3" && oggenc "target/convert.wav" -o "assets/music/$file.ogg"
done
