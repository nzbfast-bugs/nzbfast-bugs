//! `listgroups` PausedSize: NZBGet reports the paused bytes of a group,
//! and NZBGet clients (Sonarr/Radarr) call a group paused when
//! `RemainingSize == PausedSize && RemainingSize != 0`.

use super::{jr_listgroups, nzo_int};
use nzbfast_daemon::testutil::{jv, with_daemon};
use nzbkit::sync::MutexExt;
use serde_json::{Value, json};

fn group<'a>(groups: &'a Value, id: &str) -> &'a Value {
    groups
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["NZBID"] == json!(nzo_int(id)))
        .unwrap()
}

#[test]
fn paused_group_reports_its_remaining_bytes_as_paused() {
    with_daemon("lg-paused", |d| {
        let p = jv("SABnzbd_nzo_21", "p", json!({"total_bytes": 5_000_000u64}));
        p.lock_ok().paused = true;
        d.queue.lock_ok().push_back(p);
        d.queue
            .lock_ok()
            .push_back(jv("SABnzbd_nzo_22", "q", json!({"total_bytes": 7_000_000u64})));
        let gs = jr_listgroups(d);
        let p = group(&gs, "SABnzbd_nzo_21");
        assert_eq!(p["Status"], "PAUSED");
        assert_eq!(p["RemainingSizeLo"], 5_000_000);
        assert_eq!(p["PausedSizeLo"], p["RemainingSizeLo"], "paused group");
        let q = group(&gs, "SABnzbd_nzo_22");
        assert_eq!(q["PausedSizeLo"], 0, "unpaused group");
    });
}
