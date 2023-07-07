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

Comments, suggestions, bug reports and contributions welcome