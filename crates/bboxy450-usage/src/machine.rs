// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::Failure;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Machine {
    pub cpu: Option<u8>,
    pub memory: Option<u8>,
    pub disk: Option<u8>,
    pub reason: Option<String>,
}
#[derive(Default)]
pub struct Sampler {
    previous: Option<[u64; 4]>,
}
type PlatformSample = ([u64; 4], Option<u8>, Option<u8>);
pub fn cpu_delta(old: [u64; 4], new: [u64; 4]) -> Option<u8> {
    let deltas = std::array::from_fn::<_, 4, _>(|i| new[i].wrapping_sub(old[i]) & u32::MAX as u64);
    let total: u64 = deltas.iter().sum();
    (total != 0).then(|| ((total - deltas[2]) as f64 / total as f64 * 100.).round() as u8)
}
fn ratio(used: u64, total: u64) -> Option<u8> {
    (total != 0).then(|| (used.min(total) as f64 / total as f64 * 100.).round() as u8)
}
impl Sampler {
    pub fn sample(&mut self, home: &Path) -> Machine {
        match platform(home) {
            Ok((ticks, memory, disk)) => {
                let cpu = self
                    .previous
                    .and_then(|previous| cpu_delta(previous, ticks));
                self.previous = Some(ticks);
                Machine {
                    cpu,
                    memory,
                    disk,
                    reason: cpu
                        .is_none()
                        .then(|| "CPU warming up (two samples required)".into()),
                }
            }
            Err(e) => Machine {
                reason: Some(e.reason().into()),
                ..Machine::default()
            },
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn platform(_: &Path) -> Result<PlatformSample, Failure> {
    Err(Failure::Unsupported)
}

#[cfg(target_os = "macos")]
fn platform(home: &Path) -> Result<PlatformSample, Failure> {
    use std::{
        ffi::CString,
        mem::{size_of, zeroed},
        os::unix::ffi::OsStrExt,
    };
    let home = CString::new(home.as_os_str().as_bytes()).map_err(|_| Failure::Unsupported)?;
    unsafe extern "C" {
        fn mach_port_deallocate(
            task: libc::mach_port_t,
            name: libc::mach_port_t,
        ) -> libc::kern_return_t;
    }
    // All buffers are sized using the Mach ABI counts. No subprocess, filesystem traversal or UI call.
    #[allow(deprecated)]
    unsafe {
        let host = libc::mach_host_self();
        let mut processors = 0;
        let mut info: libc::processor_info_array_t = std::ptr::null_mut();
        let mut cpu_count = 0;
        let cpu_status = libc::host_processor_info(
            host,
            libc::PROCESSOR_CPU_LOAD_INFO,
            &mut processors,
            &mut info,
            &mut cpu_count,
        );
        let mut ticks = [0_u64; 4];
        if cpu_status == 0 && !info.is_null() {
            let values = std::slice::from_raw_parts(info, cpu_count as usize);
            for cpu in values.as_chunks::<4>().0 {
                for (i, value) in cpu.iter().enumerate() {
                    ticks[i] += u64::from(*value as u32);
                }
            }
            let _ = libc::vm_deallocate(
                libc::mach_task_self(),
                info as libc::vm_address_t,
                cpu_count as libc::vm_size_t * size_of::<libc::integer_t>(),
            );
        }
        let mut vm: libc::vm_statistics64_data_t = zeroed();
        let mut vm_count = libc::HOST_VM_INFO64_COUNT;
        let vm_status = libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            (&mut vm as *mut libc::vm_statistics64_data_t).cast(),
            &mut vm_count,
        );
        let page_size = libc::vm_page_size;
        let _ = mach_port_deallocate(libc::mach_task_self(), host);
        if cpu_status != libc::KERN_SUCCESS {
            return Err(Failure::Unsupported);
        }
        let mut total = 0_u64;
        let mut len = size_of::<u64>();
        let mem_status = libc::sysctlbyname(
            c"hw.memsize".as_ptr(),
            (&mut total as *mut u64).cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        );
        // Physical active + wired + compressed pages; reclaimable file cache is excluded.
        let used_pages = u64::from(vm.active_count)
            + u64::from(vm.wire_count)
            + u64::from(vm.compressor_page_count);
        let memory = if vm_status == 0 && page_size != 0 && mem_status == 0 {
            ratio(used_pages.saturating_mul(page_size as u64), total)
        } else {
            None
        };
        let mut fs: libc::statfs = zeroed();
        let disk = if libc::statfs(home.as_ptr(), &mut fs) == 0 {
            ratio(fs.f_blocks.saturating_sub(fs.f_bfree), fs.f_blocks)
        } else {
            None
        };
        Ok((ticks, memory, disk))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delta_accounts_for_all_cores_and_counter_wrap() {
        assert_eq!(cpu_delta([20, 20, 20, 20], [40, 30, 70, 40]), Some(50));
        assert_eq!(cpu_delta([0; 4], [0; 4]), None);
        assert_eq!(
            cpu_delta([u32::MAX as u64, 0, 0, 0], [1, 0, 2, 0]),
            Some(50)
        );
        assert_eq!(ratio(70, 100), Some(70));
        assert_eq!(ratio(200, 100), Some(100));
        assert_eq!(ratio(0, 0), None);
    }
}
