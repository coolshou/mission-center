use std::{cell::RefCell, collections::HashMap, sync::Arc};

use super::dbus_interface::IoMissioncenterMissionCenterGatherer;
pub use super::dbus_interface::{
    App, CpuDynamicInfo, CpuStaticInfo, GpuDynamicInfo, GpuStaticInfo, Process, ProcessState,
    ProcessUsageStats,
};
use super::{FLATPAK_APP_PATH, IS_FLATPAK};

macro_rules! dbus_call {
    ($self: ident, $method: tt, $dbus_method_name: literal $(,$args:ident)*) => {{
        use gtk::glib::g_critical;

        for i in 1..=3 {
            match $self.dbus_proxy.$method($($args)*) {
                Ok(reply) => {
                    return reply;
                }
                Err(e) => match $self.is_running() {
                    Ok(()) => {
                        if e.name() == Some("org.freedesktop.DBus.Error.NoReply") {
                            g_critical!(
                                "MissionCenter::Gatherer",
                                "DBus call '{}' timed out, on try {}",
                                $dbus_method_name, i,
                            );

                            if i == 2 {
                                g_critical!("MissionCenter::Gatherer", "Restarting Gatherer...");
                                $self.stop();
                                $self.start();
                            }
                        } else {
                            g_critical!(
                                "MissionCenter::Gatherer",
                                "DBus call '{}' failed on try {}: {}",
                                $dbus_method_name, i, e,
                            );
                        }
                    }
                    Err(exit_code) => {
                        g_critical!(
                            "MissionCenter::Gatherer",
                            "Child failed, on try {}, with exit code {}. Restarting Gatherer...",
                            i, exit_code,
                        );
                        $self.start();
                    }
                },
            }
        }

        show_error_dialog_and_exit(&format!("DBus call '{}' failed after 3 retries 😟. The app will now close.", $dbus_method_name));
    }};
}

fn show_error_dialog_and_exit(message: &str) -> ! {
    use crate::i18n::*;
    use adw::prelude::*;

    let app_window =
        crate::MissionCenterApplication::default_instance().and_then(|app| app.active_window());

    let error_dialog = adw::MessageDialog::new(
        app_window.as_ref(),
        Some("A fatal error has occurred"),
        Some(message),
    );
    error_dialog.set_modal(true);
    error_dialog.add_responses(&[("close", &i18n("_Quit"))]);
    error_dialog.set_response_appearance("close", adw::ResponseAppearance::Destructive);
    error_dialog.connect_response(None, |dialog, _| {
        dialog.close();
        std::process::exit(1);
    });
    error_dialog.present();

    std::process::exit(-1);
}

pub struct Gatherer<'a> {
    dbus_proxy: dbus::blocking::Proxy<'a, dbus::blocking::Connection>,

    command: RefCell<std::process::Command>,
    child: RefCell<Option<std::process::Child>>,
}

impl Drop for Gatherer<'_> {
    fn drop(&mut self) {
        use std::ops::DerefMut;

        if let Some(child) = self.child.borrow_mut().deref_mut() {
            let _ = child.kill();
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
                    &e
                );
                show_error_dialog_and_exit(&format!(
                    "Failed to connect to the DBus session bus: {}",
                    e
                ));
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
            child: RefCell::new(None),
        }
    }

    pub fn start(&self) {
        use gtk::glib::g_critical;
        use std::ops::DerefMut;

        std::mem::swap(
            self.child.borrow_mut().deref_mut(),
            &mut Some(match self.command.borrow_mut().spawn() {
                Ok(c) => c,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Failed to spawn Gatherer process: {}",
                        &e
                    );
                    show_error_dialog_and_exit(&format!("Failed to spawn Gatherer process: {}", e));
                }
            }),
        );

        // Let the child process start up
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    pub fn stop(&self) {
        let mut child = self.child.borrow_mut();
        if let Some(child) = child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn cpu_static_info(&self) -> CpuStaticInfo {
        dbus_call!(self, cpu_static_info, "GetCPUStaticInfo");
    }

    pub fn cpu_dynamic_info(&self) -> CpuDynamicInfo {
        dbus_call!(self, cpu_dynamic_info, "GetCPUDynamicInfo");
    }

    pub fn enumerate_gpus(&self) -> Vec<std::sync::Arc<str>> {
        dbus_call!(self, enumerate_gpus, "EnumerateGPUs");
    }

    pub fn gpu_static_info(&self, id: &str) -> GpuStaticInfo {
        dbus_call!(self, gpu_static_info, "GetGPUStaticInfo", id);
    }

    pub fn gpu_dynamic_info(&self, id: &str) -> GpuDynamicInfo {
        dbus_call!(self, gpu_dynamic_info, "GetGPUDynamicInfo", id);
    }

    pub fn processes(&self) -> HashMap<u32, Process> {
        dbus_call!(self, processes, "GetProcesses");
    }

    pub fn apps(&self) -> HashMap<Arc<str>, App> {
        dbus_call!(self, apps, "GetApps");
    }

    pub fn is_running(&self) -> Result<(), i32> {
        let mut child = self.child.borrow_mut();
        let mut child = match child.as_mut() {
            Some(child) => child,
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
