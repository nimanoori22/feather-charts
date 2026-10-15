//! Native chart work and compositor presentation feedback on Linux/Wayland.
#[cfg(target_os = "linux")]
#[path = "frame_latency_profile/native.rs"]
mod native;

fn main() {
    #[cfg(target_os = "linux")]
    native::run();
    #[cfg(not(target_os = "linux"))]
    panic!("This presentation profiler requires Linux/Wayland");
}
