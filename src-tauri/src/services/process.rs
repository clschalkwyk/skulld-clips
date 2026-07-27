use std::{
    ffi::OsString,
    io::{self, Read},
    path::Path,
    process::{Command, ExitStatus, Stdio},
    thread,
    time::Duration,
};

use wait_timeout::ChildExt;

#[derive(Debug)]
pub struct ProcessOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessError {
    Spawn,
    MissingPipe,
    Timeout,
    Wait,
    OutputLimit,
    OutputRead,
    ReaderStopped,
}

pub fn run_bounded(
    program: &Path,
    args: &[OsString],
    timeout: Duration,
    max_output_bytes: u64,
) -> Result<ProcessOutput, ProcessError> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| ProcessError::Spawn)?;

    let stdout = child.stdout.take().ok_or(ProcessError::MissingPipe)?;
    let stderr = child.stderr.take().ok_or(ProcessError::MissingPipe)?;
    let stdout_reader = thread::spawn(move || read_limited(stdout, max_output_bytes));
    let stderr_reader = thread::spawn(move || read_limited(stderr, max_output_bytes));

    let status = match child.wait_timeout(timeout) {
        Ok(Some(status)) => status,
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProcessError::Timeout);
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ProcessError::Wait);
        }
    };

    let stdout = join_reader(stdout_reader)?;
    let stderr = join_reader(stderr_reader)?;

    Ok(ProcessOutput {
        status,
        stdout,
        stderr,
    })
}

fn read_limited<R: Read>(reader: R, max_output_bytes: u64) -> Result<Vec<u8>, ProcessError> {
    let mut bytes = Vec::new();
    reader
        .take(max_output_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| match error.kind() {
            io::ErrorKind::InvalidData => ProcessError::OutputLimit,
            _ => ProcessError::OutputRead,
        })?;

    if bytes.len() as u64 > max_output_bytes {
        return Err(ProcessError::OutputLimit);
    }

    Ok(bytes)
}

fn join_reader(
    reader: thread::JoinHandle<Result<Vec<u8>, ProcessError>>,
) -> Result<Vec<u8>, ProcessError> {
    reader.join().map_err(|_| ProcessError::ReaderStopped)?
}

#[cfg(test)]
mod tests {
    use super::ProcessError;

    #[test]
    fn process_errors_do_not_contain_private_paths_or_output() {
        let errors = [
            ProcessError::Spawn,
            ProcessError::MissingPipe,
            ProcessError::Timeout,
            ProcessError::Wait,
            ProcessError::OutputLimit,
            ProcessError::OutputRead,
            ProcessError::ReaderStopped,
        ];

        for error in errors {
            let detail = format!("{error:?}");
            assert!(!detail.contains('/'));
            assert!(!detail.contains('\\'));
        }
    }
}
