#!/usr/bin/env bash
# Activation script for Linux pixi environments.
# Ensures that cargo build scripts and the dynamic linker can locate the
# system libraries installed from conda-forge (alsa-lib, libxkbcommon, etc.).

# Prepend conda-prefix pkgconfig paths so build scripts (e.g. alsa-sys,
# xkbcommon-sys) find the correct .pc files.
export PKG_CONFIG_PATH="$CONDA_PREFIX/lib/pkgconfig:$CONDA_PREFIX/share/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"

# Prepend conda-prefix lib directory so the runtime linker finds
# shared objects (.so) shipped with the conda packages.
export LD_LIBRARY_PATH="$CONDA_PREFIX/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
