# Copied from `vapoursynth-rs` and modified

# Get the arch and dir
param([String]$arch="x86_64")

$NAME = "VapourSynth"
$PY_DIR = "py-dir"
$VS_DIR = "vs-dir"

$SUFFIX = 64
$PYTHON_PKG = "python-3.9.10-embed-amd64.zip"
$VS_VERSION = "R57"

# Download Python embeddable and VapourSynth portable
$VS_PATH = "https://github.com/vapoursynth/vapoursynth/releases/download/$VS_VERSION"
curl -LO "https://www.python.org/ftp/python/3.9.10/$PYTHON_PKG"
curl -LO "$VS_PATH/VapourSynth$SUFFIX-Portable-$VS_VERSION.7z"

# Unzip Python embeddable and VapourSynth portable
7z x "$PYTHON_PKG" -o"$PY_DIR"
7z x "VapourSynth$SUFFIX-Portable-$VS_VERSION.7z" -o"$VS_DIR"

# Move all VapourSynth files inside the Python ones
Move-Item -Force -Path "$VS_DIR\*" -Destination "$PY_DIR"

# Move the VapourSynth directory into a system directory
Move-Item -Path "$PY_DIR" -Destination "C:\Program Files"
Rename-Item -Path "C:\Program Files\$PY_DIR" -NewName "$NAME"
