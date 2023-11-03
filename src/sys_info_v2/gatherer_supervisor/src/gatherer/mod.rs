use std::cell::RefCell;

use lazy_static::lazy_static;

use dbus_interface::IoMissioncenterMissionCenterGatherer;
pub use dbus_interface::{CpuDynamicInfo, CpuStaticInfo, GpuDynamicInfo, GpuStaticInfo, Process};

mod dbus_interface;

macro_rules! dbus_call {
    ($self: ident, $method: tt, $dbus_method_name: literal $(,$args:ident)*) => {{
        use gtk::glib::g_critical;

        for i in 0..2 {
            match $self.dbus_proxy.$method($($args)*) {
                Ok(reply) => {
                    return reply;
                }
                Err(e) => match $self.is_running() {
                    Ok(()) => {
                        g_critical!(
                            "MissionCenter::Gatherer",
                            "DBus call '{}' failed on try {}: {}",
                            $dbus_method_name,
                            i + 1,
                            e
                        );
                    }
                    Err(exit_code) => {
                        g_critical!(
                            "MissionCenter::Gatherer",
                            "Child failed with exit code {}. Restarting Gatherer...",
                            exit_code
                        );
                        $self.start();
                    }
                },
            }
        }

        std::process::exit(-1);
    }};
}

macro_rules! cmd_flatpak_host {
    ($cmd: expr) => {{
        use std::process::Command;

        const FLATPAK_SPAWN_CMD: &str = "/usr/bin/flatpak-spawn";

        let mut cmd = Command::new(FLATPAK_SPAWN_CMD);
        cmd.arg("--host").arg("sh").arg("-c");
        cmd.arg($cmd);

        cmd
    }};
}

lazy_static! {
    static ref IS_FLATPAK: bool = std::path::Path::new("/.flatpak-info").exists();
    static ref FLATPAK_APP_PATH: String = {
        use ini::*;

        let ini = match Ini::load_from_file("/.flatpak-info") {
            Err(_) => return "".to_owned(),
            Ok(ini) => ini,
        };

        let section = match ini.section(Some("Instance")) {
            None => panic!("Unable to find Instance section in /.flatpak-info"),
            Some(section) => section,
        };

        match section.get("app-path") {
            None => panic!("Unable to find 'app-path' key in Instance section in /.flatpak-info"),
            Some(app_path) => app_path.to_owned(),
        }
    };
}

pub struct Gatherer<'a> {
    dbus_proxy: dbus::blocking::Proxy<'a, dbus::blocking::Connection>,

    command: RefCell<std::process::Command>,
    child: Option<RefCell<std::process::Child>>,
}

impl Drop for Gatherer<'_> {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.borrow_mut().kill();
        }
    }
}

impl<'a> Gatherer<'a> {
    pub fn new() -> Self {
        use gtk::glib::g_critical;

        let mut command = if *IS_FLATPAK {
            cmd_flatpak_host!(Self::executable())
        } else {
            let mut cmd = std::process::Command::new("sh");
            cmd.arg("-c");
            cmd.arg(Self::executable());

            cmd
        };
        command
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        let connection = match dbus::blocking::Connection::new_session() {
            Ok(c) => c,
            Err(e) => {
                g_critical!(
                    "MissionCenter::Gatherer",
                    "Failed to connect to DBus session bus: {}",
                    e
                );
                std::process::exit(-1);
            }
        };

        let dbus_proxy = dbus::blocking::Proxy::new(
            "io.missioncenter.MissionCenter.Gatherer",
            "/io/missioncenter/MissionCenter/Gatherer",
            std::time::Duration::from_millis(500),
            connection,
        );

        Self {
            dbus_proxy,

            command: RefCell::new(command),
            child: None,
        }
    }

    pub fn start(&self) {
        // self.child = Some(match self.command.spawn() {
        //     Ok(c) => c,
        //     Err(e) => {
        //         g_critical!(
        //             "MissionCenter::Gatherer",
        //             "Failed to spawn Gatherer process: {}",
        //             e
        //         );
        //         std::process::exit(-1);
        //     }
        // });
        //
        // // Let the child process start up
        // std::thread::sleep(std::time::Duration::from_millis(50));
    }

    pub fn cpu_static_info(&self) -> CpuStaticInfo {
        dbus_call!(self, cpu_static_info, "GetCPUStaticInfo");
    }

    pub fn cpu_dynamic_info(&self) -> CpuDynamicInfo {
        dbus_call!(self, cpu_dynamic_info, "GetCPUDynamicInfo");
    }

    pub fn enumerate_gpus(&self) -> Vec<String> {
        dbus_call!(self, enumerate_gpus, "EnumerateGPUs");
    }

    pub fn gpu_static_info(&self, id: &str) -> GpuStaticInfo {
        dbus_call!(self, gpu_static_info, "GetGPUStaticInfo", id);
    }

    pub fn gpu_dynamic_info(&self, id: &str) -> GpuDynamicInfo {
        dbus_call!(self, gpu_dynamic_info, "GetGPUDynamicInfo", id);
    }

    pub fn processes(&self) -> Vec<Process> {
        dbus_call!(self, processes, "GetProcesses");
    }

    pub fn is_running(&self) -> Result<(), i32> {
        let mut child = match self.child.as_ref() {
            Some(child) => child.borrow_mut(),
            None => return Err(-1),
        };

        let status = match child.try_wait() {
            Ok(None) => return Ok(()),
            Ok(Some(status)) => status,
            Err(_) => {
                return Err(-1);
            }
        };

        match status.code() {
            Some(status_code) => Err(status_code),
            None => Err(-1),
        }
    }

    fn executable() -> String {
        use gtk::glib::g_debug;

        let executable_name = if *IS_FLATPAK {
            let flatpak_app_path = FLATPAK_APP_PATH.as_str();

            let cmd_status = cmd_flatpak_host!(&format!(
                "{}/bin/missioncenter-gatherer-glibc just-testing",
                flatpak_app_path
            ))
            .status();
            if let Ok(status) = cmd_status {
                if status.success() {
                    format!("{}/bin/missioncenter-gatherer-glibc", flatpak_app_path)
                } else {
                    format!("{}/bin/missioncenter-gatherer-musl", flatpak_app_path)
                }
            } else {
                format!("{}/bin/missioncenter-gatherer-musl", flatpak_app_path)
            }
        } else {
            "missioncenter-gatherer".to_string()
        };

        g_debug!(
            "MissionCenter::ProcInfo",
            "Proxy executable name: {}",
            executable_name
        );

        executable_name
    }
}
