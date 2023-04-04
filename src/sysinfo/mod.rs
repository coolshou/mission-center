use std::time::Duration;

use gtk::glib;
use sysinfo::{CpuExt, System, SystemExt};

pub fn run_cpu_usage_loop() {
    let mut sys = System::new();
    sys.refresh_cpu(); // Refreshing CPU information.

    glib::source::timeout_add(Duration::from_millis(500), move || {
        sys.refresh_cpu(); // Refreshing CPU information.
        for cpu_usage in sys.cpus().into_iter().map(|cpu| cpu.cpu_usage()) {
            println!("CPU usage: {}%", cpu_usage);
        }

        glib::Continue(true)
    });
}
