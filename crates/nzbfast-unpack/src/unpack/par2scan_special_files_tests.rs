use super::*;
use std::time::Duration;

fn temp_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nzbfast-p2special-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn with_timeout<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(Duration::from_secs(5)).ok()
}

/// A FIFO named `*.par2` in a completed job folder: verify_dir must not block.
#[test]
fn verify_dir_skips_fifo_named_par2() {
    let dir = temp_dir("fifo");
    let st = std::process::Command::new("mkfifo").arg(dir.join("evil.par2")).status().unwrap();
    assert!(st.success());
    let d = dir.clone();
    let r = with_timeout(move || verify_dir(&d).is_ok());
    assert!(r.is_some(), "verify_dir hung > 5s opening a FIFO named evil.par2");
}

/// One unreadable `.par2` must not abort the scan of the readable ones.
#[test]
fn unreadable_par2_is_skipped_not_fatal() {
    use std::os::unix::fs::PermissionsExt;
    let dir = temp_dir("perm");
    let mut good = b"PAR2\x00PKT".to_vec();
    good.resize(128, 0);
    std::fs::write(dir.join("a.par2"), &good).unwrap();
    std::fs::write(dir.join("b.vol00+01.par2"), &good).unwrap();
    std::fs::set_permissions(dir.join("b.vol00+01.par2"), std::fs::Permissions::from_mode(0o000)).unwrap();
    let r = collect_par2_bytes(&dir, 64 << 20);
    std::fs::set_permissions(dir.join("b.vol00+01.par2"), std::fs::Permissions::from_mode(0o644)).unwrap();
    let (bytes, _) = r.expect("one unreadable .par2 aborted the whole scan");
    assert_eq!(bytes.len(), 1);
}

/// A directory whose name ends in `.par2` must not abort the scan.
#[test]
fn dir_named_par2_is_ignored() {
    let dir = temp_dir("dirpar2");
    let mut good = b"PAR2\x00PKT".to_vec();
    good.resize(128, 0);
    std::fs::write(dir.join("a.par2"), &good).unwrap();
    std::fs::create_dir(dir.join("z.par2")).unwrap();
    let (bytes, _) = collect_par2_bytes(&dir, 64 << 20).expect("a dir named z.par2 aborted the scan");
    assert_eq!(bytes.len(), 1);
}
