#!/bin/bash

# shellcheck disable=SC2164

set -e

if [[ -z "$SRC_PATH" ]]; then
    echo "SRC_PATH is not set or empty"
    exit 1
fi

if [[ -z "$RECEIPE_PATH" ]]; then
    echo "RECEIPE_PATH is not set or empty"
    exit 1
fi

if [[ -z "$APPDIR_PATH" ]]; then
    echo "APPDIR_PATH is not set or empty"
    exit 1
fi

export HOME=/root
export TERM=xterm
export PATH="/usr/local/bin:/usr/local/sbin:/usr/bin:/usr/sbin:/bin:/sbin:/usr/lib/gcc/x86_64-linux-gnu/9"
export LD_LIBRARY_PATH="/usr/lib/gcc/x86_64-linux-gnu/9"

pip3 install appimage-builder

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

cd "$SRC_PATH"

apt install -y squashfs-tools zsync
appimage-builder --recipe "$RECEIPE_PATH/io.missioncenter.MissionCenter.yml" --appdir "$APPDIR_PATH"

mv Mission\ Center*.AppImage MissionCenter-x86_64.AppImage
