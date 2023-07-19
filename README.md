<img align="left"  src="https://gitlab.com/mission-center-devs/mission-center/-/raw/main/data/icons/hicolor/scalable/apps/io.missioncenter.MissionCenter.svg" alt="drawing" width="64"/> 

# Mission Center

![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0001-cpu-multi.png)

#### Monitor your CPU, Memory, Disk, Network and GPU usage

#### Features:

* Monitor overall or per-thread CPU usage
* See system process, thread, and handle count, uptime, clock speed (base and current), cache sizes
* Monitor RAM and Swap usage
* See a breakdown how the memory is being used by the system
* Monitor Disk utilization and transfer rates
* Monitor network utilization and transfer speeds
* See network interface information such as network card name, connection type (Wi-Fi or Ethernet), wireless speeds and
  frequency, hardware address, IP address
* Monitor overall GPU usage, video encoder and decoder usage, memory usage and power consumption, powered by the popular
  NVTOP project
* See a breakdown of resource usage by app and process
* Supports a minified summary view for simple monitoring
* Use OpenGL rendering for all the graphs in an effort to reduce CPU and overall resource usage
* Uses GTK4 and Libadwaita
* Written in Rust
* Flatpak first

##### Limitations (there is ongoing work to overcome all of these):

* The application currently only supports monitoring, you cannot stop processes for
  example [#1](https://gitlab.com/mission-center-devs/mission-center/-/issues/1)
* Disk utilization percentage might not be
  accurate [#2](https://gitlab.com/mission-center-devs/mission-center/-/issues/2)
* No per-process network usage [#3](https://gitlab.com/mission-center-devs/mission-center/-/issues/3)
* No per-process GPU usage [#4](https://gitlab.com/mission-center-devs/mission-center/-/issues/4)
* GPU support is experimental and only AMD and nVidia GPUs can be
  monitored [#5](https://gitlab.com/mission-center-devs/mission-center/-/issues/5)

<br/>
<p align="center">
  <a href="https://flathub.org/apps/io.missioncenter.MissionCenter"><img src="https://dl.flathub.org/assets/badges/flathub-badge-en.svg" width=200/></a>
</p>
<br/>

#### Screenshots:

*CPU overall view*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0002-cpu-overall.png)

*Memory view*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0003-memory.png)

*Disk view*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0004-disk.png)

*Ethernet and Wi-Fi view*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0005-net-wired.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0006-net-wireless.png)

*GPU view*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0007-gpu-amd.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0008-gpu-nvidia.png)

*Apps page*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0009-apps.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0010-apps-filter.png)

*Dark mode*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0011-cpu-dark.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0012-disk-dark.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0013-gpu-nvidia-dark.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0014-apps-dark.png)

*Summary view*  
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0015-cpu-summary-view.png)
![](https://gitlab.com/mission-center-devs/mission-center/-/raw/main/screenshots/0016-cpu-summary-view-dark.png)

#### Building - Native

**Requirements:**
 * Meson (version >= 0.63)
 * Rust (version >= 1.69)
 * Python3
 * Python GObject Introspection (required for Blueprint)
 * DRM development libraries
 * GBM development libraries
 * udev development libraires
 * GTK 4
 * libadwaita

**Build instructions**
```bash
# On Ubuntu 23.04 all dependencies, except for the Rust toolchain, can be installed with:
sudo apt install build-essential curl git gettext python3-pip libadwaita-1-dev python3-gi libudev-dev libdrm-dev libgbm-dev desktop-file-utils meson

meson setup _build -Dbuildtype=debug # Alternatively pass `-Dbuildtype=release` for a release build
ninja -C _build
```

If you want to run the application from the build directory (for development or debugging) some set up is required:

```bash
export PATH="$(pwd)/_build/src/proxy:$PATH"
export GSETTINGS_SCHEMA_DIR="$(pwd)/_build/data"
export HW_DB_DIR="$(pwd)/_build/data/hwdb"
export MC_RESOURCE_DIR="$(pwd)/_build/resources"

glib-compile-schemas --strict "$(pwd)/data" && mv "$(pwd)/data/gschemas.compiled" "$(pwd)/_build/data/"
```

And then to run the app:
```bash
_build/src/missioncenter
```

If you want to install the app just run:
```bash
ninja -C _build install
```

And run the app from your launcher or from the command-line:
```bash
missioncenter
```

#### Building - Flatpak

**Requirements:**
 * Flatpak
 * Flatpak-Builder

Add the `flathub` repo is not already present:
```bash
flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
```

Install the required Flatpak runtimes and SDKs:
```bash
flatpak install -y \
    org.freedesktop.Platform//22.08 \
    org.freedesktop.Sdk//22.08 \
    org.gnome.Platform//44 \
    org.gnome.Sdk//44 \
    org.freedesktop.Sdk.Extension.llvm16//22.08 \
    org.freedesktop.Sdk.Extension.rust-stable//22.08
```

Finally build a Flatpak package:
```bash
cd flatpak
flatpak-builder --repo=repo --ccache --force-clean build io.missioncenter.MissionCenter.json
flatpak build-bundle repo missioncenter.flatpak io.missioncenter.MissionCenter
```

Install the package:
```bash
flatpak uninstall -y io.missioncenter.MissionCenter
flatpak install -y missioncenter.flatpak
```

Run the app from your launcher or from the command-line:
```bash
flatpak run io.missioncenter.MissionCenter
```

<br/>

**Comments, suggestions, bug reports and contributions welcome**

