//! Issue #327: `mode=change_cat` takes SAB's comma-separated nzo_id list.

use super::*;
use crate::job::job_from_json;
use crate::testutil::test_daemon;

fn queued(d: &Arc<Daemon>, dir: &std::path::Path, id: &str, name: &str) -> Arc<Mutex<Job>> {
    let j = Arc::new(Mutex::new(
        job_from_json(&json!({
            "nzo_id": id, "name": name,
            "nzb_path": dir.join(format!("{name}.nzb")).to_string_lossy(),
            "out_dir": crate::naming::out_dir(d).join(name),
            "state": "Queued", "category": "tv",
        }))
        .unwrap(),
    ));
    d.queue.lock_ok().push_back(j.clone());
    j
}

fn change_cat(d: &Arc<Daemon>, dir: &std::path::Path, value: &str, cat: &str) -> Value {
    let mut req: tiny_http::Request = tiny_http::TestRequest::new().into();
    let cfg_path = dir.join("nzbfast.toml");
    let ctx = ApiCtx {
        cfg_path: &cfg_path,
        host_hdr: "",
        base: "",
        ua_hdr: "",
        bootstrap_apikey: false,
        via_add_only: false,
    };
    let mut params = std::collections::HashMap::new();
    params.insert("value".to_string(), value.to_string());
    params.insert("value2".to_string(), cat.to_string());
    m_change_cat(d, &mut req, &params, &ctx, &mut None).unwrap()
}

#[test]
fn change_cat_accepts_comma_separated_ids() {
    let dir = std::env::temp_dir().join(format!("nzbfast-cclist-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let d = test_daemon(&dir);
    let a = queued(&d, &dir, "SABnzbd_nzo_a", "Job.A");
    let b = queued(&d, &dir, "SABnzbd_nzo_b", "Job.B");
    let v = change_cat(&d, &dir, "SABnzbd_nzo_a, SABnzbd_nzo_b,SABnzbd_nzo_nope", "movies");
    let (ca, cb) = (a.lock_ok().category.clone(), b.lock_ok().category.clone());
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(v["status"], true, "{v}");
    assert_eq!(ca, "movies");
    assert_eq!(cb, "movies");
    let e = v["error"].as_str().unwrap_or_default();
    assert!(e.contains("SABnzbd_nzo_nope"), "the unknown id is named: {v}");
}

#[test]
fn change_cat_all_unknown_ids_is_a_failure() {
    let dir = std::env::temp_dir().join(format!("nzbfast-ccnone-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let d = test_daemon(&dir);
    let v = change_cat(&d, &dir, "SABnzbd_nzo_x,SABnzbd_nzo_y", "movies");
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(v["status"], false, "{v}");
    assert!(v["error"].as_str().unwrap_or_default().contains("SABnzbd_nzo_x"), "{v}");
}
