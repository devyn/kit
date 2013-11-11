#!/bin/sh

# This builds the following:
#
# - Cross-compiler for i586 ELF
#   (See http://wiki.osdev.org/GCC_Cross-Compiler)
#   - binutils
#   - gcc

DEST=`pwd`

die() {
  echo "!! An error occurred. Aborting."
  exit 1
}

echo ">> Installing to `pwd`"

mkdir -p src
cd src

# Set up the environment

export PREFIX="$DEST"
export TARGET=i586-elf
export PATH="$PREFIX/bin:$PATH"

# $1: Product
# $2: Version
# $3: URL
download_gnu() {
  if [ -d "$1-$2" ]; then
    echo ">> Found $1 $2"
  else
    echo ">> Downloading $1 $2"

    curl -# "$3" | tar -xz || die
  fi
}

# Download packages

TEXINFO_VERSION=4.13
TEXINFO_VERSION_ACTUAL=4.13a
BINUTILS_VERSION=2.23.2
GCC_VERSION=4.8.2

download_gnu texinfo  $TEXINFO_VERSION  "ftp://ftp.gnu.org/gnu/texinfo/texinfo-${TEXINFO_VERSION_ACTUAL}.tar.gz"
download_gnu binutils $BINUTILS_VERSION "ftp://ftp.gnu.org/gnu/binutils/binutils-${BINUTILS_VERSION}.tar.gz"
download_gnu gcc      $GCC_VERSION      "ftp://ftp.gnu.org/gnu/gcc/gcc-${GCC_VERSION}/gcc-${GCC_VERSION}.tar.gz"

# Build packages

cd ..

mkdir build
cd build

echo ">> Building texinfo"

mkdir texinfo
cd texinfo

TEXINFO_SRC=../../src/texinfo-$TEXINFO_VERSION

$TEXINFO_SRC/configure --target=$TARGET --prefix="$PREFIX"

make $MAKEFLAGS         || die
make $MAKEFLAGS install || die

cd ..

echo ">> Building binutils"

mkdir binutils
cd binutils

BINUTILS_SRC=../../src/binutils-$BINUTILS_VERSION

# --disable-nls: no native language support

$BINUTILS_SRC/configure --target=$TARGET --prefix="$PREFIX" --disable-nls

make $MAKEFLAGS         || die
make $MAKEFLAGS install || die

cd ..

echo ">> Building gcc"

mkdir gcc
cd gcc

GCC_SRC=../../src/gcc-$GCC_VERSION

# --disable-nls: no native language support
# --enable-languages=c,c++: only build C and C++ frontends
# --without-headers: do not rely on presence of C stdlib

$GCC_SRC/configure --target=$TARGET --prefix="$PREFIX" --disable-nls --enable-languages=c,c++ --without-headers

make $MAKEFLAGS all-gcc                || die
make $MAKEFLAGS all-target-libgcc      || die
make $MAKEFLAGS install-gcc            || die
make $MAKEFLAGS install-target-libgcc  || die

cd ..

echo ">> Done!"
