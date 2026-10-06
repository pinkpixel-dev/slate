//! One Slate per user. The first launch listens on a Unix socket; later
//! launches send it their file arguments and exit, so `slate notes.txt` opens
//! a tab in the window that's already running.

use std::ffi::OsStr;
use std::io::{self, Read as _, Write as _};
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use futures::channel::mpsc;

/// What a launch should do after trying to claim the socket.
pub enum Claim {
    /// This is the only Slate. Paths sent by later launches arrive on the receiver.
    Primary(mpsc::UnboundedReceiver<Vec<PathBuf>>),
    /// Another Slate took the paths. This process should exit.
    Forwarded,
}

/// `$XDG_RUNTIME_DIR/slate.sock`, or a per-user name in the temp folder.
pub fn socket_path() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) {
        Some(dir) if dir.is_absolute() => dir.join("slate.sock"),
        _ => {
            let user = std::env::var("USER").unwrap_or_else(|_| "user".into());
            std::env::temp_dir().join(format!("slate-{user}.sock"))
        }
    }
}

/// Hands `paths` to a running Slate, or becomes the one that listens.
pub fn claim(socket: &Path, paths: &[PathBuf]) -> io::Result<Claim> {
    let listener = match UnixListener::bind(socket) {
        Ok(listener) => listener,
        Err(err) if err.kind() == io::ErrorKind::AddrInUse => match send(socket, paths) {
            Ok(()) => return Ok(Claim::Forwarded),
            // Left over from a Slate that crashed or was killed.
            Err(_) => {
                std::fs::remove_file(socket)?;
                UnixListener::bind(socket)?
            }
        },
        Err(err) => return Err(err),
    };

    let (tx, rx) = mpsc::unbounded();
    std::thread::Builder::new()
        .name("slate-single-instance".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                if let Ok(paths) = receive(stream)
                    && tx.unbounded_send(paths).is_err()
                {
                    break;
                }
            }
        })?;
    Ok(Claim::Primary(rx))
}

/// Paths travel as raw bytes separated by NUL, the one byte a Linux path can't hold.
fn send(socket: &Path, paths: &[PathBuf]) -> io::Result<()> {
    let mut stream = UnixStream::connect(socket)?;
    let mut message = Vec::new();
    for path in paths {
        message.extend_from_slice(path.as_os_str().as_bytes());
        message.push(0);
    }
    stream.write_all(&message)?;
    stream.shutdown(std::net::Shutdown::Write)
}

fn receive(mut stream: UnixStream) -> io::Result<Vec<PathBuf>> {
    let mut message = Vec::new();
    stream.read_to_end(&mut message)?;
    Ok(message
        .split(|&byte| byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| PathBuf::from(OsStr::from_bytes(part)))
        .collect())
}

#[cfg(test)]
mod tests {
    use futures::StreamExt as _;

    use super::*;

    fn socket(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("slate-si-{}-{name}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn a_second_launch_forwards_its_paths() {
        let socket = socket("forward");
        let Claim::Primary(mut rx) = claim(&socket, &[]).unwrap() else {
            panic!("the first launch should listen");
        };
        let paths = vec![PathBuf::from("/tmp/a b.txt"), PathBuf::from("/tmp/ü.md")];
        assert!(matches!(claim(&socket, &paths).unwrap(), Claim::Forwarded));
        assert!(matches!(claim(&socket, &[]).unwrap(), Claim::Forwarded));

        let received = futures::executor::block_on(async { (rx.next().await, rx.next().await) });
        assert_eq!(received, (Some(paths), Some(Vec::new())));
    }

    #[test]
    fn a_stale_socket_is_replaced() {
        let socket = socket("stale");
        drop(UnixListener::bind(&socket).unwrap());
        assert!(socket.exists(), "the dead listener leaves its file behind");
        assert!(matches!(claim(&socket, &[]).unwrap(), Claim::Primary(_)));
    }
}
