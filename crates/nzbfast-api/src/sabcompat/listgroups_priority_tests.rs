//! `listgroups` reports Priority on NZBGet's scale, where Force is 900.

use super::{jr_editqueue, jr_listgroups, nzo_int};
use nzbfast_daemon::testutil::{jv, with_daemon};
use nzbkit::sync::MutexExt;
use serde_json::{Value, json};

fn prio_of(groups: &Value, id: &str) -> i64 {
    groups
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["NZBID"] == json!(nzo_int(id)))
        .unwrap()["Priority"]
        .as_i64()
        .unwrap()
}

#[test]
fn forced_job_reports_nzbget_force_priority() {
    with_daemon("lg-force", |d| {
        d.queue.lock_ok().push_back(jv("SABnzbd_nzo_11", "f", json!({"priority": 2})));
        d.queue.lock_ok().push_back(jv("SABnzbd_nzo_12", "h", json!({"priority": 1})));
        let g = jr_listgroups(d);
        assert_eq!(prio_of(&g, "SABnzbd_nzo_11"), 900, "force");
        assert_eq!(prio_of(&g, "SABnzbd_nzo_12"), 50, "high");
    });
}

#[test]
fn set_priority_900_reads_back_as_900() {
    with_daemon("lg-force-rt", |d| {
        d.queue.lock_ok().push_back(jv("SABnzbd_nzo_13", "n", json!({})));
        let mut err = None;
        let ok = jr_editqueue(
            d,
            &[json!("GroupSetPriority"), json!("900"), json!([nzo_int("SABnzbd_nzo_13")])],
            &mut err,
        );
        assert_eq!(ok, json!(true));
        assert_eq!(prio_of(&jr_listgroups(d), "SABnzbd_nzo_13"), 900);
    });
}
