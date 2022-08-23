#!/bin/bash
set -e

# Upload artifacts to my personal website

OutDir=target/artifacts

if [ "$1" = "--editor" ]; then
	FEATURES="--features editor"
else
	FEATURES=""
fi

if [ ! -e .git ]; then
	echo "Must be run from repository root"
	exit 1
fi

#
# Extract project name from Cargo.toml
#

ProjName="$(
	cargo metadata --no-deps --format-version 1 |
		  sed -n 's/.*"name":"\([^"]*\)".*/\1/p'
)"

#
# Build
#

if [ ! -e target ] ; then
    mkdir target
fi

cargo build \
	--release --no-default-features \
	$FEATURES \
	--target x86_64-pc-windows-gnu

WindowExe="$(
	cargo metadata --format-version 1 |
		sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p'
)/x86_64-pc-windows-gnu/release/$ProjName.exe"

if [ ! -e "$WindowExe" ]; then
	echo "Script is borken, it expects file to exist: $WindowExe"
	exit 1
fi

[ ! -e "$OutDir" ] && mkdir -p "$OutDir"
[ -e "$OutDir/assets" ] && rm -r "$OutDir/assets"

#
# Copy files
#

cp "$WindowExe" "$OutDir/$ProjName.exe"


cp -r assets "$OutDir/assets"

#
# Pack and upload files
#

cd "$OutDir"
zip -r klod_files.zip *
scp klod_files.zip "nicopap_nicopapch@ssh.phx.nearlyfreespeech.net:/home/public/p/klod_editor.zip"
rm klod_files.zip
cd ../..