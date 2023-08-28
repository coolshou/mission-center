/* sys_info_v2/gatherer/common/cpu.rs
 *
 * Copyright 2023 Romeo Calota
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use super::ArrayString;

mod state {
    use std::{cell::Cell, thread_local};

    thread_local! {
        pub static CPU_USAGE_CACHE: Cell<Vec<f32>> = Cell::new(vec![]);
    }
}

#[derive(Debug, Clone)]
pub struct StaticInfo {
    pub name: ArrayString,
    pub logical_cpu_count: u32,
    pub socket_count: Option<u8>,
    pub base_frequency_khz: Option<u64>,
    pub virtualization: Option<bool>,
    pub virtual_machine: Option<bool>,
    pub l1_cache: Option<usize>,
    pub l2_cache: Option<usize>,
    pub l3_cache: Option<usize>,
    pub l4_cache: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct DynamicInfo {
    pub overall_utilization_percent: f32,
    pub current_frequency_mhz: u64,
    pub temperature: Option<f32>,
    pub process_count: u32,
    pub thread_count: u32,
    pub handle_count: u32,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone)]
pub struct LogicalCpuInfo {
    pub utilization_percent: arrayvec::ArrayVec<f32, 128>,
    pub is_complete: bool,
}

impl StaticInfo {
    // Code lifted and adapted from `sysinfo` crate, found in src/linux/cpu.rs
    fn name() -> ArrayString {
        use super::ToArrayStringLossy;

        fn get_value(s: &str) -> ArrayString {
            s.split(':')
                .last()
                .map(|x| x.trim().to_array_string_lossy())
                .unwrap_or_default()
        }

        fn get_hex_value(s: &str) -> u32 {
            s.split(':')
                .last()
                .map(|x| x.trim())
                .filter(|x| x.starts_with("0x"))
                .map(|x| u32::from_str_radix(&x[2..], 16).unwrap())
                .unwrap_or_default()
        }

        fn get_arm_implementer(implementer: u32) -> Option<&'static str> {
            Some(match implementer {
                0x41 => "ARM",
                0x42 => "Broadcom",
                0x43 => "Cavium",
                0x44 => "DEC",
                0x46 => "FUJITSU",
                0x48 => "HiSilicon",
                0x49 => "Infineon",
                0x4d => "Motorola/Freescale",
                0x4e => "NVIDIA",
                0x50 => "APM",
                0x51 => "Qualcomm",
                0x53 => "Samsung",
                0x56 => "Marvell",
                0x61 => "Apple",
                0x66 => "Faraday",
                0x69 => "Intel",
                0x70 => "Phytium",
                0xc0 => "Ampere",
                _ => return None,
            })
        }

        fn get_arm_part(implementer: u32, part: u32) -> Option<&'static str> {
            Some(match (implementer, part) {
                // ARM
                (0x41, 0x810) => "ARM810",
                (0x41, 0x920) => "ARM920",
                (0x41, 0x922) => "ARM922",
                (0x41, 0x926) => "ARM926",
                (0x41, 0x940) => "ARM940",
                (0x41, 0x946) => "ARM946",
                (0x41, 0x966) => "ARM966",
                (0x41, 0xa20) => "ARM1020",
                (0x41, 0xa22) => "ARM1022",
                (0x41, 0xa26) => "ARM1026",
                (0x41, 0xb02) => "ARM11 MPCore",
                (0x41, 0xb36) => "ARM1136",
                (0x41, 0xb56) => "ARM1156",
                (0x41, 0xb76) => "ARM1176",
                (0x41, 0xc05) => "Cortex-A5",
                (0x41, 0xc07) => "Cortex-A7",
                (0x41, 0xc08) => "Cortex-A8",
                (0x41, 0xc09) => "Cortex-A9",
                (0x41, 0xc0d) => "Cortex-A17", // Originally A12
                (0x41, 0xc0f) => "Cortex-A15",
                (0x41, 0xc0e) => "Cortex-A17",
                (0x41, 0xc14) => "Cortex-R4",
                (0x41, 0xc15) => "Cortex-R5",
                (0x41, 0xc17) => "Cortex-R7",
                (0x41, 0xc18) => "Cortex-R8",
                (0x41, 0xc20) => "Cortex-M0",
                (0x41, 0xc21) => "Cortex-M1",
                (0x41, 0xc23) => "Cortex-M3",
                (0x41, 0xc24) => "Cortex-M4",
                (0x41, 0xc27) => "Cortex-M7",
                (0x41, 0xc60) => "Cortex-M0+",
                (0x41, 0xd01) => "Cortex-A32",
                (0x41, 0xd02) => "Cortex-A34",
                (0x41, 0xd03) => "Cortex-A53",
                (0x41, 0xd04) => "Cortex-A35",
                (0x41, 0xd05) => "Cortex-A55",
                (0x41, 0xd06) => "Cortex-A65",
                (0x41, 0xd07) => "Cortex-A57",
                (0x41, 0xd08) => "Cortex-A72",
                (0x41, 0xd09) => "Cortex-A73",
                (0x41, 0xd0a) => "Cortex-A75",
                (0x41, 0xd0b) => "Cortex-A76",
                (0x41, 0xd0c) => "Neoverse-N1",
                (0x41, 0xd0d) => "Cortex-A77",
                (0x41, 0xd0e) => "Cortex-A76AE",
                (0x41, 0xd13) => "Cortex-R52",
                (0x41, 0xd20) => "Cortex-M23",
                (0x41, 0xd21) => "Cortex-M33",
                (0x41, 0xd40) => "Neoverse-V1",
                (0x41, 0xd41) => "Cortex-A78",
                (0x41, 0xd42) => "Cortex-A78AE",
                (0x41, 0xd43) => "Cortex-A65AE",
                (0x41, 0xd44) => "Cortex-X1",
                (0x41, 0xd46) => "Cortex-A510",
                (0x41, 0xd47) => "Cortex-A710",
                (0x41, 0xd48) => "Cortex-X2",
                (0x41, 0xd49) => "Neoverse-N2",
                (0x41, 0xd4a) => "Neoverse-E1",
                (0x41, 0xd4b) => "Cortex-A78C",
                (0x41, 0xd4c) => "Cortex-X1C",
                (0x41, 0xd4d) => "Cortex-A715",
                (0x41, 0xd4e) => "Cortex-X3",

                // Broadcom
                (0x42, 0x00f) => "Brahma-B15",
                (0x42, 0x100) => "Brahma-B53",
                (0x42, 0x516) => "ThunderX2",

                // Cavium
                (0x43, 0x0a0) => "ThunderX",
                (0x43, 0x0a1) => "ThunderX-88XX",
                (0x43, 0x0a2) => "ThunderX-81XX",
                (0x43, 0x0a3) => "ThunderX-83XX",
                (0x43, 0x0af) => "ThunderX2-99xx",

                // DEC
                (0x44, 0xa10) => "SA110",
                (0x44, 0xa11) => "SA1100",

                // Fujitsu
                (0x46, 0x001) => "A64FX",

                // HiSilicon
                (0x48, 0xd01) => "Kunpeng-920", // aka tsv110

                // NVIDIA
                (0x4e, 0x000) => "Denver",
                (0x4e, 0x003) => "Denver 2",
                (0x4e, 0x004) => "Carmel",

                // APM
                (0x50, 0x000) => "X-Gene",

                // Qualcomm
                (0x51, 0x00f) => "Scorpion",
                (0x51, 0x02d) => "Scorpion",
                (0x51, 0x04d) => "Krait",
                (0x51, 0x06f) => "Krait",
                (0x51, 0x201) => "Kryo",
                (0x51, 0x205) => "Kryo",
                (0x51, 0x211) => "Kryo",
                (0x51, 0x800) => "Falkor-V1/Kryo",
                (0x51, 0x801) => "Kryo-V2",
                (0x51, 0x802) => "Kryo-3XX-Gold",
                (0x51, 0x803) => "Kryo-3XX-Silver",
                (0x51, 0x804) => "Kryo-4XX-Gold",
                (0x51, 0x805) => "Kryo-4XX-Silver",
                (0x51, 0xc00) => "Falkor",
                (0x51, 0xc01) => "Saphira",

                // Samsung
                (0x53, 0x001) => "exynos-m1",

                // Marvell
                (0x56, 0x131) => "Feroceon-88FR131",
                (0x56, 0x581) => "PJ4/PJ4b",
                (0x56, 0x584) => "PJ4B-MP",

                // Apple
                (0x61, 0x020) => "Icestorm-A14",
                (0x61, 0x021) => "Firestorm-A14",
                (0x61, 0x022) => "Icestorm-M1",
                (0x61, 0x023) => "Firestorm-M1",
                (0x61, 0x024) => "Icestorm-M1-Pro",
                (0x61, 0x025) => "Firestorm-M1-Pro",
                (0x61, 0x028) => "Icestorm-M1-Max",
                (0x61, 0x029) => "Firestorm-M1-Max",
                (0x61, 0x030) => "Blizzard-A15",
                (0x61, 0x031) => "Avalanche-A15",
                (0x61, 0x032) => "Blizzard-M2",
                (0x61, 0x033) => "Avalanche-M2",

                // Faraday
                (0x66, 0x526) => "FA526",
                (0x66, 0x626) => "FA626",

                // Intel
                (0x69, 0x200) => "i80200",
                (0x69, 0x210) => "PXA250A",
                (0x69, 0x212) => "PXA210A",
                (0x69, 0x242) => "i80321-400",
                (0x69, 0x243) => "i80321-600",
                (0x69, 0x290) => "PXA250B/PXA26x",
                (0x69, 0x292) => "PXA210B",
                (0x69, 0x2c2) => "i80321-400-B0",
                (0x69, 0x2c3) => "i80321-600-B0",
                (0x69, 0x2d0) => "PXA250C/PXA255/PXA26x",
                (0x69, 0x2d2) => "PXA210C",
                (0x69, 0x411) => "PXA27x",
                (0x69, 0x41c) => "IPX425-533",
                (0x69, 0x41d) => "IPX425-400",
                (0x69, 0x41f) => "IPX425-266",
                (0x69, 0x682) => "PXA32x",
                (0x69, 0x683) => "PXA930/PXA935",
                (0x69, 0x688) => "PXA30x",
                (0x69, 0x689) => "PXA31x",
                (0x69, 0xb11) => "SA1110",
                (0x69, 0xc12) => "IPX1200",

                // Phytium
                (0x70, 0x660) => "FTC660",
                (0x70, 0x661) => "FTC661",
                (0x70, 0x662) => "FTC662",
                (0x70, 0x663) => "FTC663",

                _ => return None,
            })
        }

        let mut vendor_id = ArrayString::new_const();
        let mut brand = ArrayString::new_const();
        let mut implementer = None;
        let mut part = None;

        let cpuinfo = match std::fs::read_to_string("/proc/cpuinfo") {
            Ok(s) => s,
            Err(e) => {
                println!("Gatherer: Failed to read /proc/cpuinfo: {}", e);
                return Default::default();
            }
        };

        for it in cpuinfo.split('\n') {
            if it.starts_with("vendor_id\t") {
                vendor_id = get_value(it);
            } else if it.starts_with("model name\t") {
                brand = get_value(it);
            } else if it.starts_with("CPU implementer\t") {
                implementer = Some(get_hex_value(it));
            } else if it.starts_with("CPU part\t") {
                part = Some(get_hex_value(it));
            } else {
                continue;
            }
            if (!brand.is_empty() && !vendor_id.is_empty())
                || (implementer.is_some() && part.is_some())
            {
                break;
            }
        }

        if let (Some(implementer), Some(part)) = (implementer, part) {
            match get_arm_implementer(implementer) {
                Some(s) => vendor_id = s.to_array_string_lossy(),
                None => return brand,
            }

            match get_arm_part(implementer, part) {
                Some(s) => {
                    match vendor_id.try_push(' ') {
                        Err(_) => return brand,
                        _ => {}
                    }
                    match vendor_id.try_push_str(s) {
                        Err(_) => return brand,
                        _ => {}
                    }
                    brand = vendor_id;
                }
                _ => {}
            }
        }

        brand
    }

    fn logical_cpu_count() -> u32 {
        num_cpus::get() as u32
    }

    fn socket_count() -> Option<u8> {
        use std::{fs::*, io::*};

        let mut sockets = std::collections::HashSet::new();
        sockets.reserve(4);

        let mut buf = String::new();

        let entries = match read_dir("/sys/devices/system/cpu/") {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("Gatherer: Could not read '/sys/devices/system/cpu': {}", e);
                return None;
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    eprintln!(
                        "Gatherer: Could not read entry in '/sys/devices/system/cpu': {}",
                        e
                    );
                    continue;
                }
            };

            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(e) => {
                    eprintln!(
                        "Gatherer: Could not read file type for '/sys/devices/system/cpu/{}': {}",
                        entry.file_name().to_string_lossy(),
                        e
                    );
                    continue;
                }
            };

            if !file_type.is_dir() {
                continue;
            }

            let mut file = match File::open(entry.path().join("topology/physical_package_id")) {
                Ok(file) => file,
                Err(_) => {
                    continue;
                }
            };

            buf.clear();
            match file.read_to_string(&mut buf) {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("Gatherer: Could not read '/sys/devices/system/cpu/{}/topology/physical_package_id': {}", file_name, e);
                    continue;
                }
            };

            let socket_id = match buf.trim().parse::<u8>() {
                Ok(socket_id) => socket_id,
                Err(e) => {
                    eprintln!("Gatherer: Could not read '/sys/devices/system/cpu/{}/topology/physical_package_id': {}", file_name, e);
                    continue;
                }
            };
            sockets.insert(socket_id);
        }

        if sockets.is_empty() {
            eprintln!("Gatherer: Could not determine socket count");
            None
        } else {
            Some(sockets.len() as u8)
        }
    }

    fn base_frequency_khz() -> Option<u64> {
        match std::fs::read("/sys/devices/system/cpu/cpu0/cpufreq/base_frequency") {
            Ok(content) => {
                let content = match std::str::from_utf8(&content) {
                    Ok(content) => content,
                    Err(e) => {
                        eprintln!("Gatherer: Could not read base frequency: {}", e);
                        return None;
                    }
                };

                match content.trim().parse() {
                    Ok(freq) => Some(freq),
                    Err(e) => {
                        eprintln!("Gatherer: Could not read base frequency: {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                eprintln!(
                    "Gatherer: Could not read base frequency: {}; trying /proc/cpuinfo",
                    e
                );

                let cpuinfo = match std::fs::read_to_string("/proc/cpuinfo") {
                    Ok(output) => output,
                    Err(e) => {
                        eprintln!("Gatherer: Could not read /proc/cpuinfo: {}", e);
                        return None;
                    }
                };

                let index = match cpuinfo.find("cpu MHz") {
                    Some(index) => index,
                    None => {
                        eprintln!("Gatherer: Could not find `cpu MHz` in /proc/cpuinfo",);
                        return None;
                    }
                };

                let base_frequency = match cpuinfo[index..]
                    .lines()
                    .next()
                    .map(|line| line.split(':').nth(1).unwrap_or("").trim())
                    .map(|mhz| mhz.parse::<f32>())
                {
                    None => {
                        eprintln!("Gatherer: Failed to parse `cpu MHz` in /proc/cpuinfo",);
                        return None;
                    }
                    Some(Ok(bf)) => bf,
                    Some(Err(e)) => {
                        eprintln!(
                            "Gatherer: Failed to parse `cpu MHz` in /proc/cpuinfo: {}",
                            e
                        );
                        return None;
                    }
                };

                Some((base_frequency * 1000.).round() as u64)
            }
        }
    }

    fn virtualization() -> Option<bool> {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        fn cpuid_ex<const START: u8, const END: u8>(leaf: u32, result: &mut [u32]) -> Option<()> {
            use raw_cpuid::*;

            let x = cpuid!(leaf);
            for (result_i, i) in (START..END).enumerate() {
                match i {
                    0 => {
                        result[result_i] = x.eax;
                    }
                    1 => {
                        result[result_i] = x.ebx;
                    }
                    2 => {
                        result[result_i] = x.ecx;
                    }
                    3 => {
                        result[result_i] = x.edx;
                    }
                    _ => {
                        return None;
                    }
                }
            }

            Some(())
        }

        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
        fn cpuid_ex<const _START: u8, const _END: u8>(_: u32, _: &mut [u32]) -> Option<()> {
            None
        }

        let mut signature_reg = [0u32; 3];
        let res = cpuid_ex::<1, 4>(0, &mut signature_reg);
        if res.is_none() {
            return None;
        }

        let mut features = [0_u32];
        cpuid_ex::<2, 3>(1, &mut features);

        //Is intel? Check bit5
        if signature_reg[0] == 0x756e6547
            && signature_reg[1] == 0x6c65746e
            && signature_reg[2] == 0x49656e69
        {
            return Some((features[0] & 0x20) > 0);
        }

        //Is AMD? check bit2
        if signature_reg[0] == 0x68747541
            && signature_reg[1] == 0x69746e65
            && signature_reg[2] == 0x444d4163
        {
            return Some((features[0] & 0x04) > 0);
        }

        None
    }

    fn virtual_machine() -> Option<bool> {
        use rustbus::*;
        use std::time::Duration;

        let mut rpc_con = match RpcConn::system_conn(connection::Timeout::Duration(
            Duration::from_millis(1000),
        )) {
            Ok(rpc_con) => rpc_con,
            Err(e) => {
                eprintln!(
                    "Gatherer: Failed to determine VM: Failed to connect to D-Bus: {}",
                    e
                );
                return None;
            }
        };

        let mut call = MessageBuilder::new()
            .call("Get")
            .at("org.freedesktop.systemd1")
            .on("/org/freedesktop/systemd1")
            .with_interface("org.freedesktop.DBus.Properties")
            .build();

        match call
            .body
            .push_param2("org.freedesktop.systemd1.Manager", "Virtualization")
        {
            Ok(_) => {}
            Err(e) => {
                eprintln!(
                    "Gatherer: Failed to determine VM: Failed to marshal parameters: {}",
                    e
                );
                return None;
            }
        }

        let id = match rpc_con
            .send_message(&mut call)
            .and_then(|smc| smc.write_all().map_err(|e| e.1))
        {
            Ok(id) => id,
            Err(e) => {
                eprintln!(
                    "Gatherer: Failed to determine VM: Failed to send message: {}",
                    e
                );
                return None;
            }
        };

        let message = match rpc_con.wait_response(
            id,
            connection::Timeout::Duration(Duration::from_millis(1000)),
        ) {
            Ok(message) => message,
            Err(e) => {
                eprintln!(
                    "Gatherer: Failed to determine VM: Failed to retrieve response: {}",
                    e
                );
                return None;
            }
        };

        match message.typ {
            MessageType::Error => {
                eprintln!(
                    "Gatherer: Failed to determine VM: Received error message: {}: {}",
                    message.dynheader.error_name.unwrap_or_default(),
                    message
                        .body
                        .parser()
                        .get::<&str>()
                        .unwrap_or("Unknown error")
                );
                return None;
            }
            MessageType::Reply => {
                use wire::unmarshal::traits::Variant;

                let reply = message
                    .body
                    .parser()
                    .get::<Variant>()
                    .and_then(|v| v.get::<&str>());

                return Some(reply.unwrap_or_default().len() > 0);
            }
            _ => {
                eprintln!(
                    "Gatherer: Failed to determine VM: Expected message type Reply got: {:?}",
                    message.typ
                );
            }
        }

        None
    }
}
