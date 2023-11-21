#!/bin/bash

# shellcheck disable=SC2164

apt-get update

ln -sf /usr/share/zoneinfo/Etc/UTC /etc/localtime
DEBIAN_FRONTEND=noninteractive apt-get install -y tzdata
dpkg-reconfigure --frontend noninteractive tzdata

apt-get install -y build-essential flex bison curl git gettext python3-pip python3-gi libudev-dev libdrm-dev libgbm-dev libdbus-1-dev libxslt-dev libpcre2-dev libfuse3-dev libgcrypt-dev libjpeg-turbo8-dev libpng-dev libisocodes-dev libepoxy-dev libxrandr-dev libxi-dev libxcursor-dev libxdamage-dev libxinerama-dev libgstreamer-plugins-bad1.0-dev libpixman-1-dev libfontconfig1-dev libxkbcommon-dev libcurl4-openssl-dev libyaml-dev libzstd-dev libgraphviz-dev librsvg2-2 libtiff5 shared-mime-info desktop-file-utils pkg-config gperf itstool xsltproc valac docbook-xsl
ln -sf python3 /usr/bin/python
pip3 install cmake meson ninja appimage-builder

# https://www.linuxfromscratch.org/blfs/view/stable/general/fribidi.html
# ----------------------------------------------------------------------
curl -LO https://github.com/fribidi/fribidi/releases/download/v1.0.13/fribidi-1.0.13.tar.xz
tar xvf fribidi-1.0.13.tar.xz
cd fribidi-1.0.13
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../.. && rm -rf fribidi-1.0.13*

# https://www.linuxfromscratch.org/blfs/view/stable/general/glib2.html
# --------------------------------------------------------------------
rm -rf /usr/include/glib-2.0/
curl -LO https://download.gnome.org/sources/glib/2.78/glib-2.78.1.tar.xz
tar xvf glib-2.78.1.tar.xz
cd glib-2.78.1
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/x86_64-linux-gnu \
    --buildtype=release                \
    -Dman=false
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../.. && rm -rf glib-2.78.1*

# https://www.linuxfromscratch.org/blfs/view/stable/general/gobject-introspection.html
# ------------------------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/gobject-introspection/1.76/gobject-introspection-1.76.1.tar.xz
tar xvf gobject-introspection-1.76.1.tar.xz
cd gobject-introspection-1.76.1
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../.. && rm -rf gobject-introspection-1.76.1*

# https://www.linuxfromscratch.org/blfs/view/stable/x/gdk-pixbuf.html
# -------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/gdk-pixbuf/2.42/gdk-pixbuf-2.42.10.tar.xz
tar xvf gdk-pixbuf-2.42.10.tar.xz
cd gdk-pixbuf-2.42.10
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/x86_64-linux-gnu \
    --buildtype=release                \
    -Dman=false                        \
    --wrap-mode=nofallback
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../.. && rm -rf gdk-pixbuf-2.42.10*

# https://www.linuxfromscratch.org/blfs/view/stable/x/graphene.html
# -----------------------------------------------------------------
curl -LO https://download.gnome.org/sources/graphene/1.10/graphene-1.10.8.tar.xz
tar xvf graphene-1.10.8.tar.xz
cd graphene-1.10.8
mkdir build && cd build
/usr/local/bin/meson .. --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../.. && rm -rf graphene-1.10.8*

# https://www.linuxfromscratch.org/blfs/view/stable/x/cairo.html
# --------------------------------------------------------------
curl -LO https://download.gnome.org/sources/cairo/1.17/cairo-1.17.6.tar.xz
tar xvf cairo-1.17.6.tar.xz
cd cairo-1.17.6
sed -e "/@prefix@/a exec_prefix=@exec_prefix@" \
-i util/cairo-script/cairo-script-interpreter.pc.in
./configure --prefix=/usr              \
    --libdir=/usr/lib/x86_64-linux-gnu \
    --disable-static                   \
    --enable-tee
make && make install && make DESTDIR=/src/appimage install
cd .. && rm -rf cairo-1.17.6*

# https://www.linuxfromscratch.org/blfs/view/stable/general/python-modules.html#pycairo
# -------------------------------------------------------------------------------------
curl -LO https://github.com/pygobject/pycairo/releases/download/v1.24.0/pycairo-1.24.0.tar.gz
tar xvf pycairo-1.24.0.tar.gz
cd pycairo-1.24.0
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../.. && rm -rf pycairo-1.24.0*

# https://www.linuxfromscratch.org/blfs/view/stable/general/python-modules.html#pygobject3
# ----------------------------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/pygobject/3.44/pygobject-3.44.1.tar.xz
tar xvf pygobject-3.44.1.tar.xz
cd pygobject-3.44.1
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../.. && rm -rf pygobject-3.44.1*

# https://www.linuxfromscratch.org/blfs/view/stable/general/wayland.html
# ----------------------------------------------------------------------
curl -LO https://gitlab.freedesktop.org/wayland/wayland/-/releases/1.22.0/downloads/wayland-1.22.0.tar.xz
tar xvf wayland-1.22.0.tar.xz
cd wayland-1.22.0
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release -Ddocumentation=false ..
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../../ && rm -rf wayland-1.22.0*

# https://www.linuxfromscratch.org/blfs/view/stable/general/wayland-protocols.html
# --------------------------------------------------------------------------------
curl -LO https://gitlab.freedesktop.org/wayland/wayland-protocols/-/releases/1.32/downloads/wayland-protocols-1.32.tar.xz
tar xvf wayland-protocols-1.32.tar.xz
cd wayland-protocols-1.32
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../../ && rm -rf wayland-protocols-1.32*

# https://www.linuxfromscratch.org/blfs/view/stable/x/adwaita-icon-theme.html
# ---------------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/adwaita-icon-theme/45/adwaita-icon-theme-45.0.tar.xz
tar xvf adwaita-icon-theme-45.0.tar.xz
cd adwaita-icon-theme-45.0
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../ && rm -rf adwaita-icon-theme-45.0*

# https://www.linuxfromscratch.org/blfs/view/stable/x/pango.html
# ------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/pango/1.51/pango-1.51.0.tar.xz
tar xvf pango-1.51.0.tar.xz
cd pango-1.51.0
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../../ && rm -rf pango-1.50.14*

# https://www.linuxfromscratch.org/blfs/view/stable/general/harfbuzz.html
# -----------------------------------------------------------------------
curl -LO https://github.com/harfbuzz/harfbuzz/releases/download/8.1.1/harfbuzz-8.1.1.tar.xz
tar xvf harfbuzz-8.1.1.tar.xz
cd harfbuzz-8.1.1
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/x86_64-linux-gnu \
    --buildtype=release                \
    -Dgraphite2=disabled
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../../ && rm -rf harfbuzz-8.1.1*

# https://www.linuxfromscratch.org/blfs/view/stable/x/gtk4.html
# -------------------------------------------------------------
curl -LO https://download.gnome.org/sources/gtk/4.12/gtk-4.12.3.tar.xz
tar xvf gtk-4.12.3.tar.xz
cd gtk-4.12.3
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/x86_64-linux-gnu \
    --buildtype=release                \
    -Dbroadway-backend=true            \
    -Dintrospection=enabled            \
    -Dbuild-examples=false             \
    -Dbuild-tests=false                \
    -Dbuild-demos=false                      \
    -Dbuild-testsuite=false
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../../ && rm -rf gtk-4.12.3*

# https://www.linuxfromscratch.org/blfs/view/stable/general/vala.html
# -------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/vala/0.56/vala-0.56.11.tar.xz
tar xvf vala-0.56.11.tar.xz
cd vala-0.56.11
CFLAGS=-O2 ./configure --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu
make && make install && make DESTDIR=/src/appimage install
cd ../ && rm -rf vala-0.56.11*

# AppStream
# ---------
curl -LO https://www.freedesktop.org/software/appstream/releases/AppStream-0.16.3.tar.xz
tar xvf AppStream-0.16.3.tar.xz
cd AppStream-0.16.3
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/x86_64-linux-gnu \
    --buildtype=release                \
    -Dstemming=false                   \
    -Dsystemd=false                    \
    -Dvapi=false                       \
    -Dapidocs=false                    \
    -Dinstall-docs=false
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../../ && rm -rf AppStream-0.16.3*

# https://www.linuxfromscratch.org/blfs/view/stable/x/libadwaita.html
# -------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/libadwaita/1.4/libadwaita-1.4.0.tar.xz
tar xvf libadwaita-1.4.0.tar.xz
cd libadwaita-1.4.0
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/x86_64-linux-gnu \
    --buildtype=release                \
    -Dtests=false                      \
    -Dexamples=false
ninja && ninja install && env DESTDIR=/src/appimage ninja install
cd ../../ && rm -rf libadwaita-1.4.0*

# Blueprint Compiler
# ------------------
curl -LO https://gitlab.gnome.org/jwestman/blueprint-compiler/-/archive/80aaee374d332b0c7e04a132cce9c472d6427a1e/blueprint-compiler-80aaee374d332b0c7e04a132cce9c472d6427a1e.tar.bz2
tar xvf blueprint-compiler-*.tar.bz2 && rm blueprint-compiler-*.tar.bz2
cd blueprint-compiler-*
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/x86_64-linux-gnu \
    --buildtype=release
ninja && ninja install
cd ../../ && rm -rf blueprint-compiler-*
# Patch for compatibility with Python 3.8
sed -i '1s/^/from __future__ import annotations\n/' /usr/lib/python3/dist-packages/blueprintcompiler/gir.py
sed -i '1s/^/from __future__ import annotations\n/' /usr/lib/python3/dist-packages/blueprintcompiler/ast_utils.py

# Rust
# ----
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
export PATH="$HOME/.cargo/bin:$PATH"

# https://github.com/AppImageCrafters/appimage-builder/issues/280
# ---------------------------------------------------------------
cat <<EOF > appimage-builder.patch
diff --git a/package.py b/package.py
index 792a724..d8175a4 100644
--- a/python3.8/site-packages/appimagebuilder/modules/deploy/apt/package.py
+++ b/package.py
@@ -76,7 +76,7 @@ class Package:

     def __gt__(self, other):
         if isinstance(other, Package):
-            return version.parse(self.version) > version.parse(other.version)
+            return version.parse(self.version.replace("ubuntu", "")) > version.parse(other.version.replace("ubuntu", ""))

     def __hash__(self):
         return self.__str__().__hash__()

EOF
patch -u /usr/local/lib/python3.8/dist-packages/appimagebuilder/modules/deploy/apt/package.py -i appimage-builder.patch

rm -rf /src/appimage/usr/bin

cd /src
meson setup _build -Dbuildtype=release -Dprefix=/usr
ninja -C _build && env DESTDIR=/src/appimage ninja -C _build install
glib-compile-schemas /src/appimage/usr/share/glib-2.0/schemas/

apt install -y squashfs-tools zsync
appimage-builder --recipe appimage/io.missioncenter.MissionCenter.yml --appdir appimage

mv Mission\ Center*.AppImage MissionCenter-x86_64.AppImage
