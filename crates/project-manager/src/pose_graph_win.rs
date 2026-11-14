//! Windows adapter for `pose-graph v0.1`: read-only Toolhelp32 process-basename
//! check without opening foreign process memory.

use vua_orchestrator::pose_graph::{PoseProbeUnavailable, PoseProcessProbe};

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsPoseProcessProbe;

impl PoseProcessProbe for WindowsPoseProcessProbe {
    fn is_process_running(&self, exe_name: &str) -> Result<bool, PoseProbeUnavailable> {
        #[cfg(windows)]
        {
            crate::play_session::os::has_process_named(exe_name).map_err(|_| PoseProbeUnavailable)
        }
        #[cfg(not(windows))]
        {
            let _ = exe_name;
            Err(PoseProbeUnavailable)
        }
    }
}
