//! Prints one real poll against this machine. Development aid, not shipped.
fn main() {
    let mut obs = agentpet_core::Observer::new();
    let p = obs.poll(&agentpet_core::procs::SystemProcessTable::new());
    println!("{}", serde_json::to_string_pretty(&p).unwrap());
}
