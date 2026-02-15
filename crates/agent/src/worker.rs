use crate::system::PrivilegeWrapper;

pub fn run(join_token: &str) {
    let privilege_wrapper = PrivilegeWrapper::new();

    if std::env::var_os("NANOSCALE_AGENT_SELFTEST_SUDO").is_some() {
        let _ = privilege_wrapper.run("/usr/bin/systemctl", &["status", "nanoscale-agent"]);
    }

    println!("Starting worker mode with join token: {join_token}");
}
