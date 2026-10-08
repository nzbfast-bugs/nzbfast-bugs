//! SAB's `mode=retry&value=<nzo_id>[&password=<pw>]` with an optional
//! multipart `nzbfile` upload (issue #328).
//!
//! The arm used to read `value` alone: a password sent with the retry -
//! the usual reason to retry a job that failed to unpack - was dropped,
//! and so was a replacement NZB, while the answer said `status: true`.
//! Both are applied now, and when either cannot be the request is
//! refused before anything moves, rather than retried without it.

use super::*;

pub(super) fn m_retry(
    d: &Arc<Daemon>,
    req: &mut tiny_http::Request,
    params: &std::collections::HashMap<String, String>,
    api_body: &mut Option<Vec<u8>>,
) -> Value {
    let id = params.get("value").cloned().unwrap_or_default();
    let password = params
        .get("password")
        .map(String::as_str)
        .filter(|p| !p.is_empty());
    // An upload is only looked for on a multipart request; a body with
    // a boundary but no file part is a broken upload, not "no upload".
    let boundary = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("Content-Type"))
        .and_then(|h| multipart_boundary(h.value.as_str()));
    let upload = match boundary {
        None => None,
        Some(b) => {
            let raw = api_body.take().unwrap_or_default();
            match multipart_file(&raw, &b) {
                Some((_, bytes)) => Some(bytes),
                None => {
                    return json!({"status": false, "nzo_id": id,
                        "error": "the request was an upload but carried no nzbfile"});
                }
            }
        }
    };
    // Validated BEFORE the retry: a bad NZB must not leave the job
    // re-queued against the old one.
    let replacement = match upload {
        None => None,
        Some(bytes) => match nzbkit::nzb::Nzb::parse(&bytes) {
            Ok(nzb) => Some((bytes, nzb.eager_bytes())),
            Err(e) => {
                return json!({"status": false, "nzo_id": id,
                    "error": format!("the uploaded nzbfile is not a usable NZB: {e}")});
            }
        },
    };
    let hist_job = d
        .history
        .lock_ok()
        .iter()
        .find(|j| j.lock_ok().nzo_id == id)
        .cloned();
    let Some(hist_job) = hist_job else {
        return json!({"status": false, "nzo_id": id,
            "error": "no job in history with that nzo_id"});
    };
    // The NZB is swapped in place before the retry so the re-queued job
    // reads the new one; the old bytes are kept to put back if the
    // retry itself is refused.
    let mut restore: Option<(PathBuf, Option<Vec<u8>>, u64)> = None;
    if let Some((bytes, total)) = &replacement {
        let (path, old_total) = {
            let g = hist_job.lock_ok();
            (g.nzb_path.clone(), g.total_bytes)
        };
        let old = std::fs::read(&path).ok();
        let tmp = path.with_extension("nzb.retry-tmp");
        let wrote = std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, &path));
        if let Err(e) = wrote {
            let _ = std::fs::remove_file(&tmp);
            return json!({"status": false, "nzo_id": id,
                "error": format!("the uploaded nzbfile could not be stored at {}: {e}",
                                 path.display())});
        }
        hist_job.lock_ok().total_bytes = *total;
        restore = Some((path, old, old_total));
    }
    if !d.retry(&id) {
        if let Some((path, old, old_total)) = restore {
            if let Some(old) = old {
                let _ = std::fs::write(&path, old);
            }
            hist_job.lock_ok().total_bytes = old_total;
        }
        return json!({"status": false, "nzo_id": id,
            "error": "this job could not be retried right now"});
    }
    if let Some(pw) = password {
        let queued = d
            .queue
            .lock_ok()
            .iter()
            .find(|j| j.lock_ok().nzo_id == id)
            .cloned();
        match queued {
            Some(j) => j.lock_ok().password = Some(pw.to_string()),
            None => {
                return json!({"status": false, "nzo_id": id,
                    "error": "the job was retried but the password could not be set - \
                              it is no longer in the queue"});
            }
        }
        if !d.save_queue() {
            return json!({"status": false, "nzo_id": id,
                "error": "the job was retried but the password could not be written \
                          to the queue store"});
        }
    }
    // The *arrs adopt the returned nzo_id as the new tracking id (SAB
    // may reissue; we keep it stable).
    json!({"status": true, "nzo_id": id})
}
