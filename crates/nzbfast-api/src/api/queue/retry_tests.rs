//! Issue #328: `mode=retry` honours `password` and an uploaded `nzbfile`.

use super::*;
use crate::job::job_from_json;
use crate::testutil::test_daemon;

const NZB: &[u8] = br#"<?xml version="1.0"?>
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb"><file poster="x" date="0" subject="&quot;a.bin&quot; yEnc (1/1)"><groups><group>g</group></groups><segments><segment bytes="1000" number="1">one@x</segment></segments></file></nzb>"#;

const NZB2: &[u8] = br#"<?xml version="1.0"?>
<nzb xmlns="http://www.newzbin.com/DTD/2003/nzb"><file poster="x" date="0" subject="&quot;b.bin&quot; yEnc (1/1)"><groups><group>g</group></groups><segments><segment bytes="5000" number="1">two@x</segment></segments></file></nzb>"#;

fn tmp(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("nzbfast-retry-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("temp dir");
    d
}

fn failed_job(d: &Arc<Daemon>, dir: &std::path::Path) -> std::path::PathBuf {
    let nzb = dir.join("Job.F.nzb");
    std::fs::write(&nzb, NZB).unwrap();
    let j = Arc::new(Mutex::new(
        job_from_json(&json!({
            "nzo_id": "SABnzbd_nzo_f", "name": "Job.F",
            "nzb_path": nzb.to_string_lossy(),
            "out_dir": crate::naming::out_dir(d).join("Job.F"),
            "state": "Failed", "category": "tv",
        }))
        .unwrap(),
    ));
    d.history.lock_ok().push(j);
    nzb
}

fn params(pairs: &[(&str, &str)]) -> std::collections::HashMap<String, String> {
    pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
}

fn queued_password(d: &Arc<Daemon>) -> Vec<Option<String>> {
    d.queue.lock_ok().iter().map(|j| j.lock_ok().password.clone()).collect()
}

#[test]
fn retry_sets_the_password() {
    let dir = tmp("pw");
    let d = test_daemon(&dir);
    failed_job(&d, &dir);
    let mut req: tiny_http::Request = tiny_http::TestRequest::new().into();
    let v = retry::m_retry(
        &d,
        &mut req,
        &params(&[("value", "SABnzbd_nzo_f"), ("password", "s3cret")]),
        &mut None,
    );
    let pws = queued_password(&d);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(v["status"], true, "{v}");
    assert_eq!(pws, vec![Some("s3cret".to_string())]);
}

#[test]
fn retry_replaces_the_nzb_with_the_upload() {
    let dir = tmp("nzb");
    let d = test_daemon(&dir);
    let path = failed_job(&d, &dir);
    let mut body = b"--XyZ\r\nContent-Disposition: form-data; name=\"nzbfile\"; filename=\"new.nzb\"\r\n\
Content-Type: application/x-nzb\r\n\r\n"
        .to_vec();
    body.extend_from_slice(NZB2);
    body.extend_from_slice(b"\r\n--XyZ--\r\n");
    let mut req: tiny_http::Request = tiny_http::TestRequest::new()
        .with_method(tiny_http::Method::Post)
        .with_header("Content-Type: multipart/form-data; boundary=XyZ".parse().unwrap())
        .into();
    let v = retry::m_retry(&d, &mut req, &params(&[("value", "SABnzbd_nzo_f")]), &mut Some(body));
    let now = std::fs::read(&path).unwrap();
    let total = d.queue.lock_ok().front().map(|j| j.lock_ok().total_bytes);
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(v["status"], true, "{v}");
    assert_eq!(now, NZB2, "the retried job still reads the old NZB");
    assert_eq!(total, Some(nzbkit::nzb::Nzb::parse(NZB2).unwrap().eager_bytes()));
}

#[test]
fn retry_with_a_bad_upload_is_refused_and_moves_nothing() {
    let dir = tmp("bad");
    let d = test_daemon(&dir);
    let path = failed_job(&d, &dir);
    let body = b"--XyZ\r\nContent-Disposition: form-data; name=\"nzbfile\"; filename=\"new.nzb\"\r\n\r\nnot xml\r\n--XyZ--\r\n".to_vec();
    let mut req: tiny_http::Request = tiny_http::TestRequest::new()
        .with_method(tiny_http::Method::Post)
        .with_header("Content-Type: multipart/form-data; boundary=XyZ".parse().unwrap())
        .into();
    let v = retry::m_retry(&d, &mut req, &params(&[("value", "SABnzbd_nzo_f")]), &mut Some(body));
    let now = std::fs::read(&path).unwrap();
    let qlen = d.queue.lock_ok().len();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(v["status"], false, "{v}");
    assert_eq!(now, NZB);
    assert_eq!(qlen, 0, "a refused retry re-queued the job anyway");
}
