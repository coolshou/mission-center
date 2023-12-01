use std::{cell::RefCell, collections::HashMap, sync::Arc};

pub use super::dbus_interface::{
    App, CpuDynamicInfo, CpuStaticInfo, GpuDynamicInfo, GpuStaticInfo, OpenGLApi, Process,
    ProcessUsageStats,
};
use super::dbus_interface::{IoMissioncenterMissionCenterGatherer, OrgFreedesktopDBusPeer};
use super::{FLATPAK_APP_PATH, IS_FLATPAK};

macro_rules! dbus_call {
    ($self: ident, $method: tt, $dbus_method_name: literal $(,$args:ident)*) => {{
        use gtk::glib::g_critical;

        let mut start = false;
        for i in 1..=3 {
            if start {
                $self.start();
            }

            match $self.dbus_proxy.borrow().as_ref().and_then(|f|Some(f.$method($($args)*))) {
                None => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "DBus proxy is not initialized, on try {}",
                        i,
                    );
                    if i == 3 {
                        show_error_dialog_and_exit(&format!("DBus proxy is not initialized after 3 retries 😟.\nThe app will now close."));
                    }
                    start = true;
                    continue;
                }
                Some(Ok(reply)) => {
                    return reply;
                }
                Some(Err(e)) => {
                    match $self.is_running() {
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
                }}
            }
        }

        show_error_dialog_and_exit(&format!("DBus call '{}' failed after 3 retries 😟.\nThe app will now close.", $dbus_method_name));
    }};
}

fn show_error_dialog_and_exit(message: &str) -> ! {
    use crate::i18n::*;
    use adw::prelude::*;

    let message = Arc::<str>::from(message);
    gtk::glib::idle_add_once(move || {
        let app_window =
            crate::MissionCenterApplication::default_instance().and_then(|app| app.active_window());

        let error_dialog = adw::MessageDialog::new(
            app_window.as_ref(),
            Some("A fatal error has occurred"),
            Some(message.as_ref()),
        );
        error_dialog.set_modal(true);
        error_dialog.add_responses(&[("close", &i18n("_Quit"))]);
        error_dialog.set_response_appearance("close", adw::ResponseAppearance::Destructive);
        error_dialog.connect_response(None, |dialog, _| {
            dialog.close();
            std::process::exit(-1);
        });
        error_dialog.present();
    });

    loop {}
}

pub struct Gatherer<'a> {
    dbus_proxy: RefCell<Option<dbus::blocking::Proxy<'a, dbus::blocking::Connection>>>,

    command: RefCell<std::process::Command>,
    child: RefCell<Option<std::process::Child>>,
}

impl Drop for Gatherer<'_> {
    fn drop(&mut self) {
        self.stop();
    }
}

impl<'a> Gatherer<'a> {
    pub fn new() -> Self {
        let mut command = if *IS_FLATPAK {
            const FLATPAK_SPAWN_CMD: &str = "/usr/bin/flatpak-spawn";

            let mut cmd = std::process::Command::new(FLATPAK_SPAWN_CMD);
            cmd.env_remove("LD_PRELOAD");
            cmd.arg("-v")
                .arg("--watch-bus")
                .arg("--host")
                .arg(Self::executable());

            cmd
        } else {
            let mut cmd = std::process::Command::new(Self::executable());
            cmd.env_remove("LD_PRELOAD");

            if let Some(mut appdir) = std::env::var_os("APPDIR") {
                appdir.push("/runtime/default");
                cmd.current_dir(appdir);
            }

            cmd
        };
        command
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());

        Self {
            dbus_proxy: RefCell::new(None),

            command: RefCell::new(command),
            child: RefCell::new(None),
        }
    }

    pub fn start(&self) {
        use gtk::glib::g_critical;

        self.child
            .replace(Some(match self.command.borrow_mut().spawn() {
                Ok(c) => c,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Failed to spawn Gatherer process: {}",
                        &e
                    );
                    show_error_dialog_and_exit(&format!("Failed to spawn Gatherer process: {}", e));
                }
            }));

        if self.dbus_proxy.borrow().is_none() {
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
                std::time::Duration::from_millis(5000),
                connection,
            );
            self.dbus_proxy.replace(Some(dbus_proxy));
        }

        // Let the child process start up
        for i in 0..8 {
            std::thread::sleep(std::time::Duration::from_millis(25));
            match self.ping() {
                Ok(()) => return,
                Err(e) => {
                    g_critical!(
                        "MissionCenter::Gatherer",
                        "Call to Gatherer Ping method failed on try {}: {}",
                        i,
                        e,
                    );
                }
            }
        }

        show_error_dialog_and_exit("Failed to spawn Gatherer process: Did not respond to Ping");
    }

    pub fn stop(&self) {
        use gtk::glib::g_critical;

        let mut child = self.child.borrow_mut();
        if let Some(child) = child.as_mut() {
            // Try to get the child to wake up in case it's stuck
            #[cfg(target_family = "unix")]
            unsafe {
                libc::kill(child.id() as _, libc::SIGCONT);
            }

            let _ = child.kill();
            for _ in 0..2 {
                match child.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => {
                        // Wait a bit and try again, the child process might just be slow to stop
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        continue;
                    }
                    Err(e) => {
                        g_critical!(
                            "MissionCenter::Gatherer",
                            "Failed to wait for Gatherer process to stop: {}",
                            &e
                        );

                        show_error_dialog_and_exit(&format!(
                            "Failed to wait for Gatherer process to stop: {}",
                            e
                        ));
                    }
                }
            }
        }
    }

    pub fn cpu_static_info(&self) -> CpuStaticInfo {
        dbus_call!(self, cpu_static_info, "GetCPUStaticInfo");
    }

    pub fn cpu_dynamic_info(&self) -> CpuDynamicInfo {
        dbus_call!(self, cpu_dynamic_info, "GetCPUDynamicInfo");
    }

    pub fn enumerate_gpus(&self) -> Vec<Arc<str>> {
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

    pub fn terminate_process(&self, process_id: u32) {
        dbus_call!(self, terminate_process, "TerminateProcess", process_id);
    }

    pub fn kill_process(&self, process_id: u32) {
        dbus_call!(self, kill_process, "KillProcess", process_id);
    }

    pub fn is_running(&self) -> Result<(), i32> {
        let mut child = self.child.borrow_mut();
        let child = match child.as_mut() {
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

    fn ping(&self) -> Result<(), Box<dyn std::error::Error>> {
        match self
            .dbus_proxy
            .borrow()
            .as_ref()
            .and_then(|f| Some(f.ping()))
        {
            None => Err("Not initialized".into()),
            Some(Ok(reply)) => Ok(reply),
            Some(Err(e)) => Err(e.into()),
        }
    }

    fn executable() -> String {
        use gtk::glib::g_debug;

        let exe_simple = "missioncenter-gatherer".to_owned();

        if *IS_FLATPAK {
            let flatpak_app_path = FLATPAK_APP_PATH.as_str();

            let cmd_glibc_status = cmd_flatpak_host!(&format!(
                "{}/bin/missioncenter-gatherer-glibc just-testing",
                flatpak_app_path
            ))
            .status()
            .is_ok_and(|exit_status| exit_status.success());
            if cmd_glibc_status {
                let exe_glibc = format!("{}/bin/missioncenter-gatherer-glibc", flatpak_app_path);
                g_debug!(
                    "MissionCenter::Gatherer",
                    "Gatherer executable name: {}",
                    &exe_glibc
                );
                return exe_glibc;
            }

            let cmd_musl_status = cmd_flatpak_host!(&format!(
                "{}/bin/missioncenter-gatherer-musl just-testing",
                flatpak_app_path
            ))
            .status()
            .is_ok_and(|exit_status| exit_status.success());
            if cmd_musl_status {
                let exe_musl = format!("{}/bin/missioncenter-gatherer-musl", flatpak_app_path);
                g_debug!(
                    "MissionCenter::Gatherer",
                    "Gatherer executable name: {}",
                    &exe_musl
                );
                return exe_musl;
            }
        }

        g_debug!(
            "MissionCenter::Gatherer",
            "Gatherer executable name: {}",
            &exe_simple
        );

        exe_simple
    }
}
