#!/bin/bash

# shellcheck disable=SC2164

set -e

if [[ -z "$SRC_PATH" ]]; then
    echo "SRC_PATH is not set or empty"
    exit 1
fi

if [[ -z "$OUT_PATH" ]]; then
    echo "OUT_PATH is not set or empty"
    exit 1
fi

export HOME=/root
export TERM=xterm
export PATH="/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin:/bin:/sbin:/usr/lib/gcc/$(arch)-linux-gnu/9:/usr/lib/gcc/$(arch)-linux-gnu/11"
export LD_LIBRARY_PATH="/usr/lib/gcc/$(arch)-linux-gnu/9:/usr/lib/gcc/$(arch)-linux-gnu/11"

apt-get update

ln -sf /usr/share/zoneinfo/Etc/UTC /etc/localtime
DEBIAN_FRONTEND=noninteractive apt-get install -y tzdata
dpkg-reconfigure --frontend noninteractive tzdata

apt-get install -y curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --default-toolchain=1.76.0 -y

apt-get install -y build-essential flex bison git gettext python3-pip python3-gi libudev-dev libdrm-dev libgbm-dev libdbus-1-dev libxslt-dev libpcre2-dev libfuse3-dev libgcrypt-dev libjpeg-turbo8-dev libpng-dev libisocodes-dev libepoxy-dev libxrandr-dev libxi-dev libxcursor-dev libxdamage-dev libxinerama-dev libgstreamer-plugins-bad1.0-dev libpixman-1-dev libfontconfig1-dev libxkbcommon-dev libcurl4-openssl-dev libyaml-dev libzstd-dev libgraphviz-dev librsvg2-2 libtiff5 shared-mime-info desktop-file-utils pkg-config gperf itstool xsltproc valac docbook-xsl libxml2-utils python3-packaging
ln -sf python3 /usr/bin/python
pip3 install cmake meson ninja

cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/fribidi.html
# ----------------------------------------------------------------------
curl -LO https://github.com/fribidi/fribidi/releases/download/v1.0.14/fribidi-1.0.14.tar.xz
tar xvf fribidi-1.0.14.tar.xz
cd fribidi-1.0.14
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf fribidi-1.0.14*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/glib2.html
# --------------------------------------------------------------------
rm -rf /usr/include/glib-2.0/
curl -LO https://download.gnome.org/sources/glib/2.80/glib-2.80.2.tar.xz
tar xvf glib-2.80.2.tar.xz
cd glib-2.80.2
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dselinux=disabled                 \
    -Dman-pages=disabled
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf glib-2.80.2*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/gobject-introspection.html
# ------------------------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/gobject-introspection/1.80/gobject-introspection-1.80.1.tar.xz
tar xvf gobject-introspection-1.80.1.tar.xz
cd gobject-introspection-1.80.1
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf gobject-introspection-1.80.1*
cd $OUT_PATH

# Yes, compile it again because I think there is a circular dependency with `gobject-introspection`
# https://www.linuxfromscratch.org/blfs/view/stable/general/glib2.html
# --------------------------------------------------------------------
rm -rf /usr/include/glib-2.0/
curl -LO https://download.gnome.org/sources/glib/2.80/glib-2.80.2.tar.xz
tar xvf glib-2.80.2.tar.xz
cd glib-2.80.2
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dselinux=disabled                 \
    -Dman-pages=disabled
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf glib-2.80.2*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/gdk-pixbuf.html
# -------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/gdk-pixbuf/2.42/gdk-pixbuf-2.42.12.tar.xz
tar xvf gdk-pixbuf-2.42.12.tar.xz
cd gdk-pixbuf-2.42.12
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dman=false                        \
    --wrap-mode=nofallback
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../.. && rm -rf gdk-pixbuf-2.42.12*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/graphene.html
# -----------------------------------------------------------------
curl -LO https://download.gnome.org/sources/graphene/1.10/graphene-1.10.8.tar.xz
tar xvf graphene-1.10.8.tar.xz
cd graphene-1.10.8
mkdir build && cd build
/usr/local/bin/meson .. --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../.. && rm -rf graphene-1.10.8*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/cairo.html
# --------------------------------------------------------------
curl -LO https://download.gnome.org/sources/cairo/1.17/cairo-1.17.6.tar.xz
tar xvf cairo-1.17.6.tar.xz
cd cairo-1.17.6
sed -e "/@prefix@/a exec_prefix=@exec_prefix@" -i util/cairo-script/cairo-script-interpreter.pc.in
./configure --prefix=/usr              \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --disable-static                   \
    --enable-tee
make && make install && make DESTDIR=$OUT_PATH install
cd .. && rm -rf cairo-1.17.6*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/python-modules.html#pycairo
# -------------------------------------------------------------------------------------
curl -LO https://github.com/pygobject/pycairo/releases/download/v1.26.0/pycairo-1.26.0.tar.gz
tar xvf pycairo-1.26.0.tar.gz
cd pycairo-1.26.0
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../.. && rm -rf pycairo-1.26.0*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/python-modules.html#pygobject3
# ----------------------------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/pygobject/3.48/pygobject-3.48.2.tar.xz
tar xvf pygobject-3.48.2.tar.xz
cd pygobject-3.48.2
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../.. && rm -rf pygobject-3.48.2*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/wayland.html
# ----------------------------------------------------------------------
curl -LO https://launchpad.net/ubuntu/+archive/primary/+sourcefiles/wayland/1.22.0-2.1build1/wayland_1.22.0.orig.tar.gz
tar xvf wayland_1.22.0.orig.tar.gz
cd wayland-1.22.0
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release -Ddocumentation=false ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf wayland*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/wayland-protocols.html
# --------------------------------------------------------------------------------
curl -LO https://launchpad.net/ubuntu/+archive/primary/+sourcefiles/wayland-protocols/1.36-1/wayland-protocols_1.36.orig.tar.xz
tar xvf wayland-protocols_1.36.orig.tar.xz
cd wayland-protocols-1.36
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf wayland-protocols*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/adwaita-icon-theme.html
# ---------------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/adwaita-icon-theme/46/adwaita-icon-theme-46.0.tar.xz
tar xvf adwaita-icon-theme-46.0.tar.xz
cd adwaita-icon-theme-46.0
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf adwaita-icon-theme-4*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/harfbuzz.html
# -----------------------------------------------------------------------
curl -LO https://github.com/harfbuzz/harfbuzz/releases/download/8.5.0/harfbuzz-8.5.0.tar.xz
tar xvf harfbuzz-8.5.0.tar.xz
cd harfbuzz-8.5.0
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dgraphite2=disabled               \
    -Dtests=disabled
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf harfbuzz-8.5.0*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/pango.html
# ------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/pango/1.52/pango-1.52.2.tar.xz
tar xvf pango-1.52.2.tar.xz
cd pango-1.52.2
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf pango-1.52.2*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/gtk4.html
# -------------------------------------------------------------
curl -LO https://download.gnome.org/sources/gtk/4.14/gtk-4.14.4.tar.xz
tar xvf gtk-4.14.4.tar.xz
cd gtk-4.14.4
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dbroadway-backend=true            \
    -Dintrospection=enabled            \
    -Dbuild-examples=false             \
    -Dbuild-tests=false                \
    -Dbuild-demos=false                \
    -Dbuild-testsuite=false            \
    -Dbroadway-backend=false           \
    -Dmedia-gstreamer=disabled         \
    -Dprint-cups=disabled              \
    -Dvulkan=disabled
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf gtk-4.14.4*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/general/vala.html
# -------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/vala/0.56/vala-0.56.17.tar.xz
tar xvf vala-0.56.17.tar.xz
cd vala-0.56.17
CFLAGS=-O2 ./configure --prefix=/usr --libdir=/usr/lib/$(arch)-linux-gnu
make && make install && make DESTDIR=$OUT_PATH install
cd ../ && rm -rf vala-0.56.17*
cd $OUT_PATH

# AppStream
# ---------
curl -LO https://www.freedesktop.org/software/appstream/releases/AppStream-1.0.3.tar.xz
tar xvf AppStream-1.0.3.tar.xz
cd AppStream-1.0.3
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dstemming=false                   \
    -Dsystemd=false                    \
    -Dvapi=false                       \
    -Dapidocs=false                    \
    -Dinstall-docs=false
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf AppStream-1.0.3*
cd $OUT_PATH

# https://www.linuxfromscratch.org/blfs/view/stable/x/libadwaita.html
# -------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/libadwaita/1.5/libadwaita-1.5.0.tar.xz
tar xvf libadwaita-1.5.0.tar.xz
cd libadwaita-1.5.0
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release                \
    -Dtests=false                      \
    -Dexamples=false
ninja && ninja install && env DESTDIR=$OUT_PATH ninja install
cd ../../ && rm -rf libadwaita-1*
cd $OUT_PATH

# Blueprint Compiler
# ------------------
curl -LO https://gitlab.gnome.org/jwestman/blueprint-compiler/-/archive/07e824d8e7b2273166acbe6d58e130b3487d8074/blueprint-compiler-07e824d8e7b2273166acbe6d58e130b3487d8074.tar.bz2
tar xvf blueprint-compiler-*.tar.bz2 && rm blueprint-compiler-*.tar.bz2
cd blueprint-compiler-*
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/$(arch)-linux-gnu \
    --buildtype=release
ninja && ninja install
cd ../../ && rm -rf blueprint-compiler-*
# Patch for compatibility with Python 3.8
sed -i '1s/^/from __future__ import annotations\n/' /usr/lib/python3/dist-packages/blueprintcompiler/gir.py
sed -i '1s/^/from __future__ import annotations\n/' /usr/lib/python3/dist-packages/blueprintcompiler/ast_utils.py

cd $OUT_PATH
rm -rf $OUT_PATH/usr/bin

export PATH="$HOME/.cargo/bin:$PATH"

cd $SRC_PATH
rm -rf _build && meson setup _build -Dbuildtype=release -Dprefix=/usr
ninja -C _build && env DESTDIR=$OUT_PATH ninja -C _build install

glib-compile-schemas $OUT_PATH/usr/share/glib-2.0/schemas/

cd $OUT_PATH && rm -rf usr/include/ usr/lib/{python3,$(arch)-linux-gnu/{*.la,cairo/*.la,cmake,glib-2.0,gobject-introspection,graphene-1.0,pkgconfig,libvala*,vala-*,valadoc-*}} usr/libexec/ usr/share/{aclocal,appstream,bash-completion,devhelp,gdb,gettext,glib-2.0/{codegen,dtds,gdb,gettext,valgrind},gobject-introspection-1.0,gtk-4.0/valgrind,gtk-doc,installed-tests,man,pkgconfig,thumbnailers,vala,vala-*,valadoc-*,wayland,wayland-protocols}

