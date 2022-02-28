#!/bin/bash
# Copied from `vapoursynth-rs` and modified
set -ex

if [ ! -f /.dockerenv ]; then
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

# Install VapourSynth
git clone --depth 1 --branch R57 https://github.com/vapoursynth/vapoursynth.git vs-dir
cd vs-dir
./autogen.sh
./configure --prefix=/usr
make -j2
sudo make install -j2

cd ..

# Set VapourSynth environment
sudo ldconfig /usr/local/lib
PYTHON3_LOCAL_LIB_PATH=$(echo /usr/local/lib/python3.9)
SITE=$PYTHON3_LOCAL_LIB_PATH/site-packages/vapoursynth.so
DIST=$PYTHON3_LOCAL_LIB_PATH/dist-packages/vapoursynth.so
sudo ln -s "$SITE" "$DIST"
