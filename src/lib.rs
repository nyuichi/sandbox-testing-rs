use std::{
    io::{BufRead, Read},
    net::ToSocketAddrs,
    panic::{self, panic_any},
};

use once_cell::sync::Lazy;
use serde::Deserialize;

macro_rules! function_name {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        name[..name.len() - 3].split_once("::").unwrap().1
    }};
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Entry {
    #[serde(rename = "suite")]
    Suite { event: String },
    #[serde(rename = "test")]
    Test {
        event: String,
        name: String,
        stdout: Option<String>,
    },
}

pub struct Test {
    name: String,
}

impl Test {
    fn run(&self, image: &str, opt_args: Option<&[&str]>) {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let mut args: Vec<_> = std::env::args().collect();

        let bin_path = args.remove(0);
        let bin_path_relative = bin_path.strip_prefix(manifest_dir).unwrap();
        let bin_path_sandbox = format!("/app/{}", bin_path_relative);

        let output = std::process::Command::new("docker")
            .arg("run")
            .arg("--rm")
            .args(&["-v", &format!("{}:/app:ro", manifest_dir)])
            .args(&["--env", "SANDBOX_TESTING_RUNNING_IN_SANDBOX=1"])
            .args(opt_args.unwrap_or(&[]))
            .arg(image)
            .arg(bin_path_sandbox)
            .arg(&self.name)
            .arg("--exact")
            .arg("--show-output")
            .args(&["-Zunstable-options", "--format", "json"])
            .output()
            .unwrap();

        println!("'{}'", String::from_utf8_lossy(&output.stdout));

        let stdout = String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .find_map(|entry: Entry| {
                if let Entry::Test {
                    name,
                    event,
                    stdout,
                } = entry
                {
                    if name == self.name && (event == "failed" || event == "ok") {
                        return stdout;
                    }
                }
                None
            })
            .unwrap_or_else(|| "".to_owned());
        print!("{}", stdout);

        if !output.status.success() {
            Lazy::force(&INIT);
            panic_any(PanicHandlerNoop);
        }
    }
}

struct PanicHandlerNoop;

static INIT: Lazy<()> = Lazy::new(|| {
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        if !panic_info.payload().is::<PanicHandlerNoop>() {
            hook(panic_info)
        }
    }));
});

macro_rules! test_in_docker {
    ($image: expr) => {{
        if std::env::var("SANDBOX_TESTING_RUNNING_IN_SANDBOX").is_err() {
            Test {
                name: function_name!().to_owned(),
            }
            .run($image, None);
            return;
        }
    }};

    ($image: expr, $args: expr) => {{
        if std::env::var("SANDBOX_TESTING_RUNNING_IN_SANDBOX").is_err() {
            Test {
                name: function_name!().to_owned(),
            }
            .run($image, Some($args));
            return;
        }
    }};
}

#[test]
fn test() {
    test_in_docker!("ubuntu:latest", &["--env", "HELLO=WORLD"]);

    assert!(std::process::Command::new("sh")
        .args(&["-c", ": > /etc/resolv.conf"])
        .status()
        .unwrap()
        .success());

    assert!("www.google.com:443".to_socket_addrs().is_err());

    assert!(std::process::Command::new("sh")
        .args(&["-c", "echo 8.8.8.8 > /etc/resolv.conf"])
        .status()
        .unwrap()
        .success());

    assert!("www.google.com:443".to_socket_addrs().is_ok());
}

#[cfg(test)]
mod tests {
    #[test]
    fn deserialize_cargo_unittest_output() {
        let output = r##"{ "type": "suite", "event": "started", "test_count": 2 }
        { "type": "test", "event": "started", "name": "run_in_docker" }
        { "type": "test", "event": "started", "name": "tests::it_works" }
        { "type": "test", "name": "tests::it_works", "event": "ok" }
        { "type": "test", "name": "run_in_docker", "event": "failed", "stdout": "\nthread 'run_in_docker' panicked at 'test run_in_docker failed', src/lib.rs:41:13\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n" }
        { "type": "suite", "event": "failed", "passed": 1, "failed": 1, "allowed_fail": 0, "ignored": 0, "measured": 0, "filtered_out": 0, "exec_time": 0.532127403 }"##;
        for line in output.lines() {
            let _entry: super::Entry = serde_json::from_str(line).unwrap();
        }
    }
}
