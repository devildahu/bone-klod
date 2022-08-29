#!/bin/bash
set -e
set -o xtrace

# Upload artifacts to my personal website

if [ ! -e .git ]; then
	echo "Must be run from repository root"
	exit 1
fi

if [ "$1" = "--debug" ]; then
	BuildDir="debug"
	ReleaseFlag=""
else
	BuildDir="release"
	ReleaseFlag="--release"
fi


#
# Extract project name from Cargo.toml
#

ProjName="$(
	cargo metadata --no-deps --format-version 1 |
		  sed -n 's/.*"name":"\([^"]*\)".*/\1/p'
)"

rm -r target/build_artifacts || true
mkdir target/build_artifacts

#
# Build
#

for target in x86_64-pc-windows-gnu x86_64-unknown-linux-gnu ; do
	OutDir=target/build/$target
	[ ! -e "$OutDir" ] && mkdir -p "$OutDir"
	[ -e "$OutDir/assets" ] && rm -r "$OutDir/assets"
	
	if [[ "$target" =~ "windows" ]] ; then
		Extension=".exe"
	else
		Extension=""
	fi

	for features in "--no-default-features" "--no-default-features --features editor" ; do
		if [[ "$features" =~ "editor" ]] ; then
			Trail="-editor"
		else
			Trail=""
		fi
		
		#
		# Compilation
		#

		cargo build $ReleaseFlag $features --target $target

		GameExecutable="$(
			cargo metadata --format-version 1 |
				sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p'
		)/$target/$BuildDir/${ProjName}${Extension}"

		if [ ! -e "$GameExecutable" ]; then
			echo "Script is borken, it expects file to exist: $GameExecutable"
			exit 1
		fi
		cp "$GameExecutable" "$OutDir/${ProjName}${Trail}${Extension}"
	done

	cp -r assets "$OutDir/assets"
	cp README.md "$OutDir/README.md"
	cp -r licenses "$OutDir/licenses"

	#
	# Pack files
	#

	cd "$OutDir"
	zip -r klod-$target.zip *
	mv klod-$target.zip ../../build_artifacts
	cd ../../..

done

./scripts/wasm_build.sh $1
cd target/wasm_package
zip -r klod-wasm.zip *
mv klod-wasm.zip ../build_artifacts
cd ../..