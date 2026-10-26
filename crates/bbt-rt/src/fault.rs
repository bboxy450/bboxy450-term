//! Bounded R5 instruments. Every file operation executes on the blocking pool.
use std::{
    io::{self, Read, Write},
    os::fd::FromRawFd,
    time::{Duration, Instant},
};

/// CPU consumed by the calling thread, distinct from elapsed frame phases.
pub fn thread_cpu_time() -> io::Result<Duration> {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: clock_gettime writes one valid timespec. The thread CPU clock is
    // a bounded counter read, not file, socket or synchronization IO.
    if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut value) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(Duration::new(value.tv_sec as u64, value.tv_nsec as u32))
}

/// Host load witness for accepting or discarding an R5 timing run.
pub fn host_load_average() -> io::Result<[f64; 3]> {
    let mut values = [0.; 3];
    // SAFETY: getloadavg receives space for exactly the three requested values.
    if unsafe { libc::getloadavg(values.as_mut_ptr(), 3) } != 3 {
        return Err(io::Error::last_os_error());
    }
    Ok(values)
}

#[derive(Debug)]
pub struct StalledRead {
    pub bytes: Vec<u8>,
    /// Duration for which the application read future remained incomplete.
    pub elapsed: Duration,
}

/// Hold a real file-descriptor read pending for `delay`: the blocking worker
/// reads a pipe-backed File; a separate task releases its bytes afterward.
/// This injects an application file-read stall, not slow physical storage.
pub async fn stalled_file_read(delay: Duration) -> io::Result<StalledRead> {
    stalled_file_read_until(async { tokio::time::sleep(delay).await }).await
}

/// Hold a real read until a caller-controlled signal. The future is first
/// polled after the IO worker has created the pipe, so tests can prove progress
/// while a read is pending without racing a wall-clock delay.
pub async fn stalled_file_read_until(
    release: impl std::future::Future<Output = ()> + Send,
) -> io::Result<StalledRead> {
    crate::assert_not_ui_thread();
    let (ready, started) = tokio::sync::oneshot::channel();
    let (writer, receive) = tokio::sync::oneshot::channel();
    let read = crate::handle().spawn_blocking(move || -> io::Result<Vec<u8>> {
        crate::assert_not_ui_thread();
        let mut descriptors = [-1; 2];
        // SAFETY: pipe writes two descriptors, each immediately wrapped in one
        // owning File. Error leaves no descriptor owned by this function.
        if unsafe { libc::pipe(descriptors.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let mut input = unsafe { std::fs::File::from_raw_fd(descriptors[0]) };
        let output = unsafe { std::fs::File::from_raw_fd(descriptors[1]) };
        writer
            .send(output)
            .map_err(|_| io::Error::other("R5 writer cancelled"))?;
        let _ = ready.send(Instant::now());
        let mut bytes = Vec::new();
        input.read_to_end(&mut bytes)?;
        Ok(bytes)
    });
    let start = started
        .await
        .map_err(|_| io::Error::other("R5 read failed to start"))?;
    let output = receive
        .await
        .map_err(|_| io::Error::other("R5 pipe failed"))?;
    release.await;
    crate::handle()
        .spawn_blocking(move || {
            let mut output = output;
            output.write_all(b"R5 stalled file read released\n")
        })
        .await
        .map_err(io::Error::other)??;
    let bytes = read.await.map_err(io::Error::other)??;
    Ok(StalledRead {
        bytes,
        elapsed: start.elapsed(),
    })
}
