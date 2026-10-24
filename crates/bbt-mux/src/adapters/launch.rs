use super::HarnessRecord;
use crate::{LaunchSpec, registry::SeatRecord, store::Result};
pub use bboxy450_harness_wake_up::launch::{CODEX_HOOK_EVENTS, exec_when_ready};
use std::path::{Path, PathBuf};

pub struct LaunchContext<'a> {
    pub seat: &'a SeatRecord,
    pub cwd: &'a Path,
    pub role_prompt: &'a str,
    pub executable: &'a Path,
    pub overlay_parent: &'a Path,
    pub user_home: &'a Path,
    pub user_zdotdir: Option<&'a Path>,
    pub socket: &'a Path,
    pub capability: [u8; 32],
    pub resume: bool,
}
pub struct LaunchOptions {
    pub cwd: PathBuf,
    pub role_prompt: String,
    pub executable: PathBuf,
    pub user_home: PathBuf,
    pub user_zdotdir: Option<PathBuf>,
    pub resume: bool,
}
pub struct LaunchPlan {
    pub spec: LaunchSpec,
    pub overlay: PathBuf,
    pub harness_session_id: Option<String>,
    pub notes: Vec<&'static str>,
}
impl LaunchPlan {
    /// The owner calls this only after process exit, never at close-grace start.
    pub fn cleanup(self) -> Result<()> {
        bboxy450_harness_wake_up::launch::cleanup_overlay(
            &self.overlay,
            self.spec
                .companion
                .as_ref()
                .map(|c| c.ready_socket.as_path()),
        )?;
        Ok(())
    }
}
pub fn plan(record: &HarnessRecord, context: &LaunchContext<'_>) -> Result<LaunchPlan> {
    bbt_rt::assert_not_ui_thread();
    record.validate()?;
    plan_neutral(&super::mapping::harness(record), context)
}
pub(super) fn plan_neutral(
    record: &bboxy450_harness_wake_up::HarnessRecord,
    context: &LaunchContext<'_>,
) -> Result<LaunchPlan> {
    super::validate_role(&record.kind, &context.seat.role)?;
    let skill = bboxy450_harness_wake_up::launch::render_skill(
        include_str!("skills/bboxy450/SKILL.md"),
        &serde_json::to_string(&context.seat.role.name)?,
        &serde_json::to_string(&context.seat.role.class)?,
    );
    let plan = bboxy450_harness_wake_up::launch::plan(
        record,
        &bboxy450_harness_wake_up::LaunchContext {
            id: context.seat.id,
            generation: context.seat.generation,
            name: &context.seat.name,
            role_name: &context.seat.role.name,
            harness_session_id: context.seat.harness_session_id.as_deref(),
            skill: &skill,
            cwd: context.cwd,
            role_prompt: context.role_prompt,
            executable: context.executable,
            overlay_parent: context.overlay_parent,
            user_home: context.user_home,
            user_zdotdir: context.user_zdotdir,
            socket: context.socket,
            capability: context.capability,
            resume: context.resume,
        },
    )?;
    let plan = super::mapping::launch_plan(plan, context.seat.id, context.seat.generation);
    plan.spec.validate()?;
    Ok(plan)
}
