#!/bin/bash
# Copied from `vapoursynth-rs` and modified
set -ex

if [ -z $GITHUB_ACTIONS ]; then
    echo "Needs to run in Action"
    exit 1
fi

# Install zimg
git clone --depth 1 --branch release-3.0.3 https://github.com/sekrit-twc/zimg.git
cd zimg
./autogen.sh
./configure --prefix=/usr
make -j2
sudo make install -j2

cd ..
rm -rf zimg

# Install VapourSynth
git clone --depth 1 --branch R57 https://github.com/vapoursynth/vapoursynth.git vs-dir
cd vs-dir
./autogen.sh
./configure --prefix=/usr
make -j2
sudo make install -j2

python setup.py sdist -d sdist
mkdir empty
cd empty
pip install vapoursynth --no-index --find-links ../sdist
cd ..

cd ..

