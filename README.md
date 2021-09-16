# sandbox-testing-rs

`sandbox-testing` is a test harness for running unit tests defined with `#[test]` attribute in a docker container.
This crate enables us to safely and securely test rust codes that modify the running environment or require the test runner to change system settings during testing.

## Example

Testing if the implementation of `ToSocketAddrs` for `String` correctly refers to `/etc/resolv.conf`.

```rust
#[test]
fn test() {
    sandbox_testing::test_in_docker!("ubuntu:latest");

    assert!("www.google.com:443".to_socket_addrs().is_ok());

    std::process::Command::new("sh")
        .args(&["-c", ": > /etc/resolv.conf"])
        .status()
        .unwrap();

    assert!("www.google.com:443".to_socket_addrs().is_err());
}
```

#### License

<sup>
Licensed under <a href="LICENSE-MIT">MIT license</a>.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in msgpack-schema by you shall be licensed as above, without any additional terms or conditions.
</sub>
