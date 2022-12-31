// Copyright 2021-Present (c) Raja Lehtihet & Wael El Oraiby
//
// Redistribution and use in source and binary forms, with or without
// modification, are permitted provided that the following conditions are met:
//
// 1. Redistributions of source code must retain the above copyright notice,
// this list of conditions and the following disclaimer.
//
// 2. Redistributions in binary form must reproduce the above copyright notice,
// this list of conditions and the following disclaimer in the documentation
// and/or other materials provided with the distribution.
//
// 3. Neither the name of the copyright holder nor the names of its contributors
// may be used to endorse or promote products derived from this software without
// specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
// ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
// LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
// CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
// SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
// INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
// CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
// ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
// POSSIBILITY OF SUCH DAMAGE.
use dbus::blocking::Connection;
use std::fs::File;

mod sensor_proxy;
use sensor_proxy::*;

use daemonize::*;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // First open up a connection to the session bus.
    let conn = Connection::new_system()?;

    // Second, create a wrapper struct around the connection that makes it easy
    // to send method calls to a specific destination and path.
    let proxy = conn.with_proxy(
        "net.hadess.SensorProxy",
        "/net/hadess/SensorProxy",
        Duration::from_millis(5000),
    );

    let stdout = File::create("/tmp/kde-autorotate.out").unwrap();
    let stderr = File::create("/tmp/kde-autorotate.err").unwrap();

    let daemonize = Daemonize::new()
        .pid_file("/tmp/kde-autorotate.pid") // Every method except `new` and `start`
        .chown_pid_file(true) // is optional, see `Daemonize` documentation
        .working_directory("/tmp") // for default behaviour.
        // .user("nobody")
        // .group("daemon") // Group name
        // .group(2) // or group id.
        .umask(0o777) // Set umask, `0o027` by default.
        .stdout(stdout) // Redirect stdout to `/tmp/daemon.out`.
        .stderr(stderr) // Redirect stderr to `/tmp/daemon.err`.
        .exit_action(|| println!("Executed before master process exits"))
        .privileged_action(|| "Executed before drop privileges");

    match daemonize.start() {
        Ok(_) => println!("Success, daemonized"),
        Err(e) => {
            eprintln!("Error, {}", e);
            return Err("Couldn't daemonize it".into());
        }
    }

    let mut last = String::new();
    loop {
        let current = proxy.accelerometer_orientation()?;
        if current != last {
            println!("orientation: {}", current);
            last = current;
            let xrandr_orientation = match last.as_str() {
                "normal" => Some("normal"),
                "left-up" => Some("left"),
                "right-up" => Some("right"),
                "bottom-up" => Some("inverted"),
                x => {
                    eprintln!("Invalid rotation: {}", x);
                    None
                }
            };

            match xrandr_orientation {
                Some(rotation) => {
                    let output = std::process::Command::new("xrandr")
                        .args(["--output", "eDP-1", "--rotate", rotation])
                        .output()?;

                    let stdout = String::from_utf8(output.stdout)?;
                    let stderr = String::from_utf8(output.stderr)?;
                    println!(
                        "execution: {}\nstderr: {}, stdout: {}",
                        output.status, stderr, stdout
                    )
                }
                _ => (),
            }
        }

        std::thread::sleep(Duration::from_millis(500));
    }
}
