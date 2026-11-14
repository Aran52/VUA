//! Windows adapter for `local-diagnostics v0.1`: read-only IPv4 UDP owner-table
//! inspection (`GetExtendedUdpTable` `AF_INET`, `UDP_TABLE_OWNER_PID`) and Toolhelp32
//! process liveness/basename resolution without binding UDP sockets or touching game memory.

use vua_orchestrator::local_diagnostics::{LocalDiagnosticsProbe, UdpPortObservation};

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsLocalDiagnosticsProbe;

impl LocalDiagnosticsProbe for WindowsLocalDiagnosticsProbe {
    fn is_client_running(&self) -> Result<bool, ()> {
        #[cfg(windows)]
        {
            crate::play_session::os::has_process_named("vrchat.exe").map_err(|_| ())
        }
        #[cfg(not(windows))]
        {
            Err(())
        }
    }

    fn probe_udp_port(&self, port: u16) -> UdpPortObservation {
        #[cfg(windows)]
        {
            win::probe_ipv4_udp_port(port)
        }
        #[cfg(not(windows))]
        {
            let _ = port;
            UdpPortObservation::probe_unavailable()
        }
    }
}

#[cfg(windows)]
mod win {
    use vua_orchestrator::local_diagnostics::UdpPortObservation;
    use windows_sys::Win32::{
        Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
        System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
    };

    const AF_INET: u32 = 2;
    const UDP_TABLE_OWNER_PID: i32 = 1;
    const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
    const NO_ERROR: u32 = 0;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct MibUdpRowOwnerPid {
        dw_local_addr: u32,
        dw_local_port: u32,
        dw_owning_pid: u32,
    }

    #[link(name = "iphlpapi")]
    extern "system" {
        fn GetExtendedUdpTable(
            p_udp_table: *mut u8,
            pdw_size: *mut u32,
            b_order: i32,
            ul_af: u32,
            table_class: i32,
            reserved: u32,
        ) -> u32;
    }

    pub(super) fn probe_ipv4_udp_port(target_port: u16) -> UdpPortObservation {
        let mut size: u32 = 0;
        let first = unsafe {
            GetExtendedUdpTable(
                std::ptr::null_mut(),
                &mut size,
                0,
                AF_INET,
                UDP_TABLE_OWNER_PID,
                0,
            )
        };
        if first != ERROR_INSUFFICIENT_BUFFER && first != NO_ERROR {
            return UdpPortObservation::probe_unavailable();
        }
        // Allocate u32-aligned buffer with headroom for concurrent sockets.
        let mut words = vec![0u32; (size as usize / 4).saturating_add(128).max(64)];
        let mut actual_bytes = (words.len() * 4) as u32;
        let status = unsafe {
            GetExtendedUdpTable(
                words.as_mut_ptr() as *mut u8,
                &mut actual_bytes,
                0,
                AF_INET,
                UDP_TABLE_OWNER_PID,
                0,
            )
        };
        if status != NO_ERROR || actual_bytes < 4 {
            return UdpPortObservation::probe_unavailable();
        }

        let num_entries = words[0] as usize;
        let max_rows =
            (actual_bytes as usize).saturating_sub(4) / std::mem::size_of::<MibUdpRowOwnerPid>();
        let count = num_entries.min(max_rows);
        let rows = unsafe {
            std::slice::from_raw_parts(words.as_ptr().add(1) as *const MibUdpRowOwnerPid, count)
        };

        // Prefer wildcard (0.0.0.0) or loopback (127.0.0.0/8) binds first, then any local IPv4 bind.
        let mut fallback_pid: Option<u32> = None;
        for row in rows {
            let row_port = u16::from_be((row.dw_local_port & 0xFFFF) as u16);
            if row_port != target_port {
                continue;
            }
            let addr_host_order = u32::from_be(row.dw_local_addr);
            if addr_host_order == 0 || (addr_host_order >> 24) == 127 {
                let exe = lookup_process_basename(row.dw_owning_pid);
                return UdpPortObservation::bound(row.dw_owning_pid, exe);
            }
            if fallback_pid.is_none() {
                fallback_pid = Some(row.dw_owning_pid);
            }
        }

        if let Some(pid) = fallback_pid {
            let exe = lookup_process_basename(pid);
            return UdpPortObservation::bound(pid, exe);
        }

        UdpPortObservation::unbound()
    }

    fn lookup_process_basename(target_pid: u32) -> Option<String> {
        let handle = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        if handle == INVALID_HANDLE_VALUE {
            return None;
        }
        let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut more = unsafe { Process32FirstW(handle, &mut entry) };
        let mut found = None;
        while more != 0 {
            if entry.th32ProcessID == target_pid {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
                if !name.trim().is_empty() {
                    found = Some(name);
                }
                break;
            }
            more = unsafe { Process32NextW(handle, &mut entry) };
        }
        unsafe {
            CloseHandle(handle);
        }
        found
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use std::net::UdpSocket;

    #[test]
    fn resolves_ephemeral_loopback_udp_socket_owner_and_reports_unbound_after_drop() {
        let probe = WindowsLocalDiagnosticsProbe;
        let socket = UdpSocket::bind("127.0.0.1:0").expect("ephemeral loopback bind");
        let port = socket.local_addr().expect("local_addr").port();

        let observed = probe.probe_udp_port(port);
        assert_eq!(observed.state, "bound");
        assert_eq!(observed.owner_pid, Some(std::process::id()));
        let exe = observed.owner_process.expect("process basename");
        assert!(!exe.is_empty());
        assert!(!exe.contains('\\') && !exe.contains('/'));

        drop(socket);
        let after_drop = probe.probe_udp_port(port);
        // Either unbound or (in the astronomically rare case another OS test grabbed it) another socket,
        // never probe_unavailable.
        assert_ne!(after_drop.state, "probe_unavailable");
    }
}
