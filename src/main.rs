use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{stderr, stdout, Write};
use std::path::Path;
use std::process;

fn valid_extension(ext: Option<&OsStr>) -> Option<&str> {
    match ext?.to_str() {
        Some("gif") => Some(".gif"),
        Some("jpg") => Some(".jpg"),
        Some("jpeg") => Some(".jpeg"),
        Some("png") => Some(".png"),
        Some("svg") => Some(".svg"),
        Some("webm") => Some(".webm"),
        Some("webp") => Some(".webp"),
        Some("mp4") => Some(".mp4"),
        Some("icc") => Some(".icc"),
        _ => None,
    }
}

fn execute() -> Option<()> {
    let mut args = env::args();

    if args.len() < 2 {
        return None;
    }

    let _name = args.next()?;
    let (prog, invocation) = (args.next()?, args);
    let mut replacements: Vec<(String, String)> = Vec::new();

    let args: Vec<String> = invocation
        .map(|arg| {
            let path = Path::new(&arg);

            if arg.starts_with('/') && path.exists() {
                match valid_extension(path.extension()) {
                    Some(realext) => {
                        let filename = format!("{}{}", replacements.len(), realext);

                        replacements.push((filename.clone(), arg));

                        filename
                    }
                    None => arg,
                }
            } else {
                arg
            }
        })
        .collect();

    let mut body: Vec<String> = vec![base64::encode(&prog), String::from("\n")];

    for arg in args {
        body.push(base64::encode(arg));
        body.push(String::from(","));
    }

    body.push(String::from("\n"));

    for rep in &replacements {
        let name = &rep.0;
        let file = &rep.1;

        body.push(String::from(name));
        body.push(String::from(":"));
        body.push(base64::encode(fs::read(file).ok()?));
        body.push(String::from("\n"));
    }

    let endpoint = env::var("FIBERGLASS_URL").ok()?;
    let response = ureq::post(&endpoint).send_string(&body.join("")).ok()?;

    if response.status() != 200 {
        return None;
    }

    let response_body = response.into_string().ok()?;
    let mut exit_status: i32 = 1;
    let mut program_stdout: Vec<u8> = vec![];
    let mut program_stderr: Vec<u8> = vec![];

    for (index, line) in response_body.lines().enumerate() {
        match index {
            0 => exit_status = line.parse().ok()?,
            1 => program_stdout = base64::decode(line).ok()?,
            2 => program_stderr = base64::decode(line).ok()?,
            n => {
                let target = &replacements[n - 3].1;
                fs::write(target, base64::decode(line).ok()?).ok()?;
            }
        };
    }

    stdout().write_all(&program_stdout).ok()?;
    stderr().write_all(&program_stderr).ok()?;

    match exit_status {
        0 => Some(()),
        _ => None,
    }
}

fn main() {
    match execute() {
        Some(_) => process::exit(0),
        None => process::exit(1),
    };
}
