use circular_buffer::CircularBuffer;
use cod::prelude::*;
use errata::FallibleExt;
use size_format::SizeFormatterSI;

use std::process::{Stdio, Command};
use std::io::{BufRead, BufReader};

#[errata::catch]
fn main() {
    let version = get_version();

    let mut dd = Command::new("dd");
    dd.arg("status=progress")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut i = 0;
    let mut infile = None;
    let mut outfile = None;
    for arg in std::env::args().skip(1) {
        if i < 3 && arg == "--version" {
            println!("ddr version {}", env!("CARGO_PKG_VERSION"));
            println!("dd version {}.{}", version.0, version.1);
            return Default::default(); // ok exit status
        }

        match i {
            0 => {
                dd.arg(format!("if={arg}"));
                infile = Some(arg);
            }
            1 => {
                dd.arg(format!("of={arg}"));
                outfile = Some(arg);
            }
            2 => {
                if arg != "--" {
                    errata::error!("too many arguments (to pass flags to dd, use `--`)");
                }
            }
            _ => { dd.arg(arg); }
        }

        i += 1;
    }

    if i < 2 {
        errata::error!("expected at least 2 arguments, got {i}");
    }

    let infile = infile.unwrap();
    let outfile = outfile.unwrap();
    println!("transferring `{infile}` to `{outfile}`");

    let total_bytes = std::fs::metadata(infile).fail("failed to get length of infile").len();
    let total_size = SizeFormatterSI::new(total_bytes);

    let mut dd = dd.spawn().fail("failed to spawn dd");
    std::thread::sleep(std::time::Duration::from_secs(1));
    if let Ok(status) = dd.try_wait() {
        std::io::copy(dd.stderr.as_mut().unwrap(), &mut std::io::stdout())
            .fail("failed to read dd's stderr");

        return status.unwrap_or_default();
    }

    let mut other_lines = CircularBuffer::<5, String>::new();
    for line in BufReader::new(dd.stderr.take().unwrap()).lines() {
        let Ok(line) = line else {
            color::fg(3);
            println!("failed to read output from dd");
            color::de::fg();
            continue;
        };

        if let Some(progress) = parse_progress(&line) {
            goto::up(2);
            color::fg(6);
            cod::println!(
                " {} / {} ({}%)\n {} secs, {}",
                SizeFormatterSI::new(progress.bytes),
                total_size,
                progress.bytes as f64 / total_bytes as f64,
                progress.elapsed_secs,
                progress.throughput,
            );
            color::de::fg();
        } else {
            other_lines.push_back(line);
        }

        for line in &other_lines {
            cod::println!("{line}");
        }

        goto::up(other_lines.len() as u32);

        cod::flush();
    }

    dd.wait().fail("failed to spawn dd")
}

#[derive(Debug, Clone, Copy)]
struct Progress<'a> {
    bytes: u64,
    elapsed_secs: f32,
    throughput: &'a str,
}

fn parse_progress(line: &str) -> Option<Progress<'_>> {
    let bytes = line.split_once(' ')?.0.parse().ok()?;
    let (rest, throughput) = {
        let (r, t) = line.rsplit_once(',')?;
        (r.trim(), t.trim())
    };

    let elapsed_secs = rest.rsplit_once(',')?.1.trim().strip_suffix(" s")?.parse().ok()?;

    Some(Progress {
        bytes,
        elapsed_secs,
        throughput,
    })
}

fn get_version() -> (u16, u16) {
    let dd = Command::new("dd").arg("--version")
        .output()
        .fail("failed to get dd's version");

    let out = String::from_utf8(dd.stdout).fail("`dd --version` gave invalid output");
    let line = out.lines().next().fail("`dd --version` gave invalid output");
    let (rest, end) = line.trim().rsplit_once('.').fail("`dd --version` gave invalid output");
    let (_, start) = rest.rsplit_once(' ').fail("`dd --version` gave invalid output");

    let major = start.parse().fail("`dd --version` gave invalid output");
    let minor = end.parse().fail("`dd --version` gave invalid output");

    (major, minor)
}
