use libc::timespec;

pub fn timespec_to_f64(ts: &timespec) -> f64 {
    ts.tv_sec as f64 + (ts.tv_nsec as f64) * 1e-9
}