#!/bin/bash

apt-get update

ln -sf /usr/share/zoneinfo/Etc/UTC /etc/localtime
DEBIAN_FRONTEND=noninteractive apt-get install -y tzdata
dpkg-reconfigure --frontend noninteractive tzdata

apt-get install -y build-essential flex bison curl git gettext python3-pip python3-gi libudev-dev libdrm-dev libgbm-dev libdbus-1-dev libxslt-dev libpcre2-dev libfuse3-dev libgcrypt-dev libjpeg-turbo8-dev libpng-dev libisocodes-dev libepoxy-dev libxrandr-dev libxi-dev libxcursor-dev libxdamage-dev libxinerama-dev libgstreamer-plugins-bad1.0-dev libpixman-1-dev libfontconfig1-dev libxkbcommon-dev libcurl4-openssl-dev libyaml-dev libzstd-dev libgraphviz-dev librsvg2-2 libtiff5 shared-mime-info desktop-file-utils pkg-config gperf itstool xsltproc valac docbook-xsl
pip3 install cmake meson ninja

# https://www.linuxfromscratch.org/blfs/view/stable/general/fribidi.html
# ----------------------------------------------------------------------
curl -LO https://github.com/fribidi/fribidi/releases/download/v1.0.13/fribidi-1.0.13.tar.xz
tar xvf fribidi-1.0.13.tar.xz
cd fribidi-1.0.13
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=/libraries ninja install
cd ../.. && rm -rf fribidi-1.0.13*

# https://www.linuxfromscratch.org/blfs/view/stable/general/glib2.html
# --------------------------------------------------------------------
rm -rf /usr/include/glib-2.0/
curl -LO https://download.gnome.org/sources/glib/2.76/glib-2.76.4.tar.xz
tar xvf glib-2.76.4.tar.xz
cd glib-2.76.4
mkdir build && cd build
/usr/local/bin/meson setup ..          \
    --prefix=/usr                      \
    --libdir=/usr/lib/x86_64-linux-gnu \
    --buildtype=release                \
    -Dman=false
ninja && ninja install && env DESTDIR=/libraries ninja install
cd ../.. && rm -rf glib-2.76.4*

# https://www.linuxfromscratch.org/blfs/view/stable/general/gobject-introspection.html
# ------------------------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/gobject-introspection/1.76/gobject-introspection-1.76.1.tar.xz
tar xvf gobject-introspection-1.76.1.tar.xz
cd gobject-introspection-1.76.1
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release
ninja && ninja install && env DESTDIR=/libraries ninja install
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
ninja && ninja install && env DESTDIR=/libraries ninja install
cd ../.. && rm -rf gdk-pixbuf-2.42.10*

# https://www.linuxfromscratch.org/blfs/view/stable/x/graphene.html
# -----------------------------------------------------------------
curl -LO https://download.gnome.org/sources/graphene/1.10/graphene-1.10.8.tar.xz
tar xvf graphene-1.10.8.tar.xz
cd graphene-1.10.8
mkdir build && cd build
/usr/local/bin/meson .. --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release
ninja && ninja install && env DESTDIR=/libraries ninja install
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
make && make install && make DESTDIR=/libraries install
cd .. && rm -rf cairo-1.17.6*

# https://www.linuxfromscratch.org/blfs/view/stable/general/python-modules.html#pycairo
# -------------------------------------------------------------------------------------
curl -LO https://github.com/pygobject/pycairo/releases/download/v1.24.0/pycairo-1.24.0.tar.gz
tar xvf pycairo-1.24.0.tar.gz
cd pycairo-1.24.0
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=/libraries ninja install
cd ../.. && rm -rf pycairo-1.24.0*

# https://www.linuxfromscratch.org/blfs/view/stable/general/python-modules.html#pygobject3
# ----------------------------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/pygobject/3.44/pygobject-3.44.1.tar.xz
tar xvf pygobject-3.44.1.tar.xz
cd pygobject-3.44.1
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=/libraries ninja install
cd ../.. && rm -rf pygobject-3.44.1*

# https://www.linuxfromscratch.org/blfs/view/stable/general/wayland.html
# ----------------------------------------------------------------------
curl -LO https://gitlab.freedesktop.org/wayland/wayland/-/releases/1.22.0/downloads/wayland-1.22.0.tar.xz
tar xvf wayland-1.22.0.tar.xz
cd wayland-1.22.0
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release -Ddocumentation=false ..
ninja && ninja install && env DESTDIR=/libraries ninja install
cd ../../ && rm -rf wayland-1.22.0*

# https://www.linuxfromscratch.org/blfs/view/stable/general/wayland-protocols.html
# --------------------------------------------------------------------------------
curl -LO https://gitlab.freedesktop.org/wayland/wayland-protocols/-/releases/1.32/downloads/wayland-protocols-1.32.tar.xz
tar xvf wayland-protocols-1.32.tar.xz
cd wayland-protocols-1.32
mkdir build && cd build
/usr/local/bin/meson setup --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu --buildtype=release ..
ninja && ninja install && env DESTDIR=/libraries ninja install
cd ../../ && rm -rf wayland-protocols-1.32*

# https://www.linuxfromscratch.org/blfs/view/stable/x/adwaita-icon-theme.html
# ---------------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/adwaita-icon-theme/44/adwaita-icon-theme-44.0.tar.xz
tar xvf adwaita-icon-theme-44.0.tar.xz
cd adwaita-icon-theme-44.0
./configure --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu
make && make install && make DESTDIR=/libraries install
cd ../ && rm -rf adwaita-icon-theme-44.0*

# https://icon-theme.freedesktop.org/releases/hicolor-icon-theme-0.17.tar.xz
# --------------------------------------------------------------------------
curl -LO https://icon-theme.freedesktop.org/releases/hicolor-icon-theme-0.17.tar.xz
tar xvf hicolor-icon-theme-0.17.tar.xz
cd hicolor-icon-theme-0.17
./configure --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu
make && make install && make DESTDIR=/libraries install
cd ../ && rm -rf hicolor-icon-theme-0.17*

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
    -Ddemos=false                      \
    -Dbuild-testsuite=false
ninja && ninja install && env DESTDIR=/libraries ninja install
cd ../../ && rm -rf gtk-4.12.3*

# https://www.linuxfromscratch.org/blfs/view/stable/general/vala.html
# -------------------------------------------------------------------
curl -LO https://download.gnome.org/sources/vala/0.56/vala-0.56.11.tar.xz
tar xvf vala-0.56.11.tar.xz
cd vala-0.56.11
CFLAGS=-O2 ./configure --prefix=/usr --libdir=/usr/lib/x86_64-linux-gnu
make && make install && make DESTDIR=/libraries install
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
ninja && ninja install && env DESTDIR=/libraries ninja install
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
ninja && ninja install && env DESTDIR=/libraries ninja install
cd ../../ && rm -rf libadwaita-1.4.0*
