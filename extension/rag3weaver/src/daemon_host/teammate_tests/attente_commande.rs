//! **A long command is not said to wait for an index** (the night's essays, 21 Nov. 2026: `cargo check` in a task was
//! said « an index is probably holding the write » eight minutes long). A file of its own, as c7 asks of every new
//! witness: never in the body of a shared tests module.
use crate::daemon_host::teammate_tools::the_waiting_line;

/// For `run_command` and `wait_output`, the line says the command is still running; for the other tools, the index
/// stays the likely cause.
#[test]
fn a_long_command_is_not_said_to_wait_for_an_index() {
    for tool in ["run_command", "wait_output"] {
        let l = the_waiting_line(tool, 20);
        assert!(l.contains("the command is still running") && !l.contains("index"), "{l}");
        assert!(l.starts_with(&format!("\"{tool}\" has been waiting for 20 s")) && l.ends_with("the call goes on, nothing is abandoned"), "{l}");
    }
    assert!(the_waiting_line("read_file", 20).contains("an index is probably holding the write"));
}
