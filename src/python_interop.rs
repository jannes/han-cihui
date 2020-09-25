use std::io::Write;
use std::process::{Command, Stdio};
use std::str;

pub fn run_python(program_str: &str) -> String {
    let mut process = match Command::new("python")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(process) => process,
        Err(err) => panic!("could not spawn python: {}", err),
    };

    let stdin = process.stdin.as_mut().unwrap();
    if let Err(why) = stdin.write(program_str.as_bytes()) {
        panic!("couldn't write to python stdin: {}", why)
    }

    let output = match process.wait_with_output() {
        Ok(output) => output,
        Err(why) => panic!("couldn't read python stdout: {}", why),
    };

    if output.status.success() {
        str::from_utf8(&output.stdout)
            .expect("expected python output to be utf8")
            .to_string()
    } else {
        panic!("python return status non zero")
    }
}
