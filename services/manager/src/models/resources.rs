//! Read-only host resource sampling.
//!
//! Linux hosts are sampled from `/proc` and `statvfs`. Unsupported platforms
//! deliberately return mock/fallback data so Windows development stays usable.

use std::collections::HashMap;
use std::time::Instant;
#[cfg(target_os = "linux")]
use std::{fs, path::Path, time::Duration};

use crate::mock;
use crate::models::domain::ResourceSample;

const KB_PER_GB: f64 = 1024.0 * 1024.0;
const BYTES_PER_GB: f64 = 1024.0 * 1024.0 * 1024.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CpuCounters {
    idle: u64,
    total: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MemInfo {
    mem_total_kb: u64,
    mem_available_kb: u64,
    mem_free_kb: u64,
    swap_total_kb: u64,
    swap_free_kb: u64,
}

pub async fn sample(cluster_dir: &str, started_at: Instant) -> ResourceSample {
    #[cfg(target_os = "linux")]
    {
        match linux_sample(cluster_dir, started_at).await {
            Ok(sample) => sample,
            Err(err) => {
                tracing::warn!("host resource sampling failed, using fallback data: {err}");
                fallback(started_at, "fallback")
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        let _ = cluster_dir;
        fallback(started_at, "fallback")
    }
}

fn fallback(started_at: Instant, source: &str) -> ResourceSample {
    let mut sample = mock::resources();
    sample.source = source.to_string();
    sample.manager_uptime_secs = started_at.elapsed().as_secs();
    sample
}

#[cfg(target_os = "linux")]
async fn linux_sample(
    cluster_dir: &str,
    started_at: Instant,
) -> Result<ResourceSample, Box<dyn std::error::Error + Send + Sync>> {
    let mem = parse_meminfo(&fs::read_to_string("/proc/meminfo")?)?;
    let load = parse_loadavg(&fs::read_to_string("/proc/loadavg")?)?;
    let cpu_a = parse_cpu_counters(&fs::read_to_string("/proc/stat")?)?;
    tokio::time::sleep(Duration::from_millis(120)).await;
    let cpu_b = parse_cpu_counters(&fs::read_to_string("/proc/stat")?)?;
    let cpu_pct = cpu_usage_pct(cpu_a, cpu_b);
    let disk = disk_usage(cluster_dir)
        .or_else(|| disk_usage("/"))
        .unwrap_or_default();
    let system_uptime_secs = parse_uptime_secs(&fs::read_to_string("/proc/uptime")?).ok();

    Ok(ResourceSample {
        source: "host".into(),
        ram_used_gb: gb_from_kb(mem.mem_total_kb.saturating_sub(mem.mem_available_kb)),
        ram_total_gb: gb_from_kb(mem.mem_total_kb),
        ram_available_gb: gb_from_kb(mem.mem_available_kb),
        cpu_pct,
        swap_used_gb: gb_from_kb(mem.swap_total_kb.saturating_sub(mem.swap_free_kb)),
        swap_total_gb: gb_from_kb(mem.swap_total_kb),
        disk_used_gb: round1(disk.used_gb),
        disk_total_gb: round1(disk.total_gb),
        disk_free_gb: round1(disk.free_gb),
        ark_proc_mem_gb: round1(ark_process_memory_gb()),
        load1: load.0,
        load5: load.1,
        load15: load.2,
        manager_uptime_secs: started_at.elapsed().as_secs(),
        system_uptime_secs,
    })
}

fn gb_from_kb(kb: u64) -> f64 {
    round1(kb as f64 / KB_PER_GB)
}

fn round1(n: f64) -> f64 {
    (n * 10.0).round() / 10.0
}

fn parse_meminfo(input: &str) -> Result<MemInfo, String> {
    let fields: HashMap<&str, u64> = input
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let key = parts.next()?.trim_end_matches(':');
            let value = parts.next()?.parse::<u64>().ok()?;
            Some((key, value))
        })
        .collect();

    let get = |key: &str| {
        fields
            .get(key)
            .copied()
            .ok_or_else(|| format!("missing {key} in /proc/meminfo"))
    };
    Ok(MemInfo {
        mem_total_kb: get("MemTotal")?,
        mem_available_kb: fields
            .get("MemAvailable")
            .copied()
            .unwrap_or(get("MemFree")?),
        mem_free_kb: get("MemFree")?,
        swap_total_kb: fields.get("SwapTotal").copied().unwrap_or(0),
        swap_free_kb: fields.get("SwapFree").copied().unwrap_or(0),
    })
}

fn parse_loadavg(input: &str) -> Result<(f64, f64, f64), String> {
    let mut parts = input.split_whitespace();
    let one = parts
        .next()
        .ok_or_else(|| "missing load1".to_string())?
        .parse::<f64>()
        .map_err(|e| e.to_string())?;
    let five = parts
        .next()
        .ok_or_else(|| "missing load5".to_string())?
        .parse::<f64>()
        .map_err(|e| e.to_string())?;
    let fifteen = parts
        .next()
        .ok_or_else(|| "missing load15".to_string())?
        .parse::<f64>()
        .map_err(|e| e.to_string())?;
    Ok((one, five, fifteen))
}

fn parse_cpu_counters(input: &str) -> Result<CpuCounters, String> {
    let line = input
        .lines()
        .find(|line| line.starts_with("cpu "))
        .ok_or_else(|| "missing aggregate cpu line in /proc/stat".to_string())?;
    let values: Vec<u64> = line
        .split_whitespace()
        .skip(1)
        .map(str::parse)
        .collect::<Result<_, _>>()
        .map_err(|e| format!("invalid cpu counter: {e}"))?;
    if values.len() < 4 {
        return Err("cpu line has too few counters".into());
    }
    let idle = values.get(3).copied().unwrap_or(0) + values.get(4).copied().unwrap_or(0);
    let total = values.iter().sum();
    Ok(CpuCounters { idle, total })
}

fn cpu_usage_pct(before: CpuCounters, after: CpuCounters) -> u32 {
    let total_delta = after.total.saturating_sub(before.total);
    if total_delta == 0 {
        return 0;
    }
    let idle_delta = after.idle.saturating_sub(before.idle);
    let busy = total_delta.saturating_sub(idle_delta);
    ((busy as f64 / total_delta as f64) * 100.0).round() as u32
}

#[derive(Debug, Clone, Copy, Default)]
struct DiskUsage {
    total_gb: f64,
    used_gb: f64,
    free_gb: f64,
}

#[cfg(target_os = "linux")]
fn disk_usage(path: &str) -> Option<DiskUsage> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let p = Path::new(path);
    let target = if p.exists() { p } else { Path::new("/") };
    let c_path = CString::new(target.as_os_str().as_bytes()).ok()?;
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `c_path` is a valid nul-terminated path and `stat` points to
    // writable memory for libc to fill. The call is read-only.
    let rc = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if rc != 0 {
        return None;
    }
    // SAFETY: statvfs returned success, so libc initialized the struct.
    let stat = unsafe { stat.assume_init() };
    let block_size = stat.f_frsize as f64;
    let total = stat.f_blocks as f64 * block_size;
    let free = stat.f_bavail as f64 * block_size;
    let used = total - free;
    Some(DiskUsage {
        total_gb: total / BYTES_PER_GB,
        used_gb: used / BYTES_PER_GB,
        free_gb: free / BYTES_PER_GB,
    })
}

fn parse_uptime_secs(input: &str) -> Result<u64, String> {
    let first = input
        .split_whitespace()
        .next()
        .ok_or_else(|| "missing uptime".to_string())?;
    let uptime = first.parse::<f64>().map_err(|e| e.to_string())?;
    Ok(uptime.round() as u64)
}

#[cfg(target_os = "linux")]
fn ark_process_memory_gb() -> f64 {
    let Ok(entries) = fs::read_dir("/proc") else {
        return 0.0;
    };
    let mut kb = 0u64;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name.to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let proc_dir = entry.path();
        if !looks_like_ark_process(&proc_dir) {
            continue;
        }
        if let Some(rss) = read_vmrss_kb(&proc_dir.join("status")) {
            kb = kb.saturating_add(rss);
        }
    }
    kb as f64 / KB_PER_GB
}

#[cfg(target_os = "linux")]
fn looks_like_ark_process(proc_dir: &Path) -> bool {
    let comm = fs::read_to_string(proc_dir.join("comm")).unwrap_or_default();
    if comm.contains("ShooterGame") || comm.contains("ArkAscended") {
        return true;
    }
    let cmdline = fs::read(proc_dir.join("cmdline")).unwrap_or_default();
    let cmdline = String::from_utf8_lossy(&cmdline);
    cmdline.contains("ShooterGame") || cmdline.contains("ArkAscended")
}

#[cfg(target_os = "linux")]
fn read_vmrss_kb(path: &Path) -> Option<u64> {
    let status = fs::read_to_string(path).ok()?;
    status.lines().find_map(|line| {
        line.strip_prefix("VmRSS:")
            .and_then(|rest| rest.split_whitespace().next())
            .and_then(|value| value.parse::<u64>().ok())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meminfo_sample() {
        let parsed = parse_meminfo(
            "MemTotal:       32768000 kB\nMemFree:         2048000 kB\nMemAvailable:   8192000 kB\nSwapTotal:       8388608 kB\nSwapFree:        6291456 kB\n",
        )
        .unwrap();

        assert_eq!(parsed.mem_total_kb, 32768000);
        assert_eq!(parsed.mem_available_kb, 8192000);
        assert_eq!(parsed.mem_free_kb, 2048000);
        assert_eq!(parsed.swap_total_kb, 8388608);
        assert_eq!(parsed.swap_free_kb, 6291456);
    }

    #[test]
    fn parses_loadavg_sample() {
        let parsed = parse_loadavg("0.12 0.34 0.56 1/234 12345").unwrap();
        assert_eq!(parsed, (0.12, 0.34, 0.56));
    }

    #[test]
    fn calculates_cpu_pct_from_proc_stat_counters() {
        let a = parse_cpu_counters("cpu  100 0 100 800 0 0 0 0 0 0\n").unwrap();
        let b = parse_cpu_counters("cpu  150 0 150 900 0 0 0 0 0 0\n").unwrap();
        assert_eq!(cpu_usage_pct(a, b), 50);
    }
}
