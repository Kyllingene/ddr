use errata::FallibleExt;

use std::process::Command;

sarge::sarge! {
    Args,

    version: bool,
}

#[errata::catch]
fn main() {
    let (args, files) = Args::parse().fail("invalid arguments");
    let version = get_version();

    if args.version {
        println!("ddr version {}", env!("CARGO_PKG_VERSION"));
        println!("dd version {}.{}", version.0, version.1);

        return;
    }

    let [infile, outfile] = &files[1..] else {
        errata::error!("expected at least two arguments, got {}", files.len() - 1);
    };

    println!("transferring `{infile}` to `{outfile}`");

    let mut dd = Command::new("dd")
        .arg(format!("if={infile}"))
        .arg(format!("of={outfile}"))
        .arg("progress=status");
}

#[derive(Debug, Clone, Copy)]
struct Progress<'a> {
    bytes: u64,
    elapsed_secs: f32,
    bandwidth: &'a str,
}

fn parse_progress(line: &str) -> Option<Progress<'_>> {
    let bytes = line.split_once(' ')?.0.parse().ok()?;
    let (rest, bandwidth) = {
        let (r, b) = line.rsplit_once(',')?;
        (r.trim(), b.trim())
    };

    let elapsed_secs = rest.rsplit_once(',')?.1.trim().strip_suffix(" s")?.parse().ok()?;

    Some(Progress {
        bytes,
        elapsed_secs,
        bandwidth,
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
