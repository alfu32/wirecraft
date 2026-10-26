use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
    sync::{Mutex, broadcast, watch},
};
use wirecraft_core::{Error, Result};
#[derive(Clone)]
pub struct Jobs {
    inner: Arc<Mutex<BTreeMap<String, Job>>>,
    events: broadcast::Sender<Value>,
}
struct Job {
    state: Value,
    cancel: watch::Sender<bool>,
}
impl Jobs {
    pub fn new(events: broadcast::Sender<Value>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BTreeMap::new())),
            events,
        }
    }
    pub async fn start(
        &self,
        executable: &str,
        args: &[String],
        cwd: &Path,
        timeout: Duration,
    ) -> Result<Value> {
        let cwd = cwd.canonicalize()?;
        let mut jobs = self.inner.lock().await;
        if jobs
            .values()
            .filter(|j| j.state["status"] == "running")
            .count()
            >= 4
        {
            return Err(Error::new("busy", "Four build jobs are already running"));
        }
        let mut cmd = Command::new(executable);
        cmd.args(args)
            .current_dir(cwd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        cmd.process_group(0);
        crate::process::terminate_with_parent(&mut cmd);
        let mut child = cmd.spawn().map_err(|e| {
            Error::new(
                "tool_unavailable",
                format!("Cannot start {executable}: {e}"),
            )
        })?;
        let pid = child.id();
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let (tx, mut rx) = watch::channel(false);
        let state = json!({"id":id,"status":"running","pid":pid,"executable":executable});
        jobs.insert(
            id.clone(),
            Job {
                state: state.clone(),
                cancel: tx,
            },
        );
        drop(jobs);
        let manager = self.clone();
        let job_id = id.clone();
        let _ = self.events.send(json!({"type":"job.started","data":state}));
        tokio::spawn(async move {
            let out = tokio::spawn(capture(stdout));
            let err = tokio::spawn(capture(stderr));
            let (status, exit_code) = tokio::select! {
                result=child.wait()=>match result{Ok(exit)=>(if exit.success(){"succeeded"}else{"failed"},exit.code()),Err(_)=>("failed",None)},
                _=rx.changed()=>{kill_group(pid);let _=child.kill().await;let _=child.wait().await;("cancelled",None)},
                _=tokio::time::sleep(timeout)=>{kill_group(pid);let _=child.kill().await;let _=child.wait().await;("timed_out",None)},
            };
            // Clean up children which inherited the process group and output pipes.
            kill_group(pid);
            let stdout = out.await.unwrap_or_else(|_| "Output reader failed".into());
            let stderr = err.await.unwrap_or_else(|_| "Output reader failed".into());
            let state = json!({"id":job_id,"status":status,"exitCode":exit_code,"stdout":stdout,"stderr":stderr});
            if let Some(j) = manager.inner.lock().await.get_mut(&job_id) {
                j.state = state.clone();
            }
            let _ = manager
                .events
                .send(json!({"type":"job.finished","data":state}));
        });
        Ok(state)
    }
    pub async fn list(&self) -> Value {
        json!({"jobs":self.inner.lock().await.values().map(|j|j.state.clone()).collect::<Vec<_>>()})
    }
    pub async fn get(&self, id: &str) -> Result<Value> {
        self.inner
            .lock()
            .await
            .get(id)
            .map(|j| j.state.clone())
            .ok_or_else(|| Error::new("not_found", "Unknown build job"))
    }
    pub async fn cancel(&self, id: &str) -> Result<Value> {
        let jobs = self.inner.lock().await;
        let job = jobs
            .get(id)
            .ok_or_else(|| Error::new("not_found", "Unknown build job"))?;
        if job.state["status"] == "running" {
            let _ = job.cancel.send(true);
        }
        Ok(json!({"id":id,"cancellationRequested":true}))
    }
    pub async fn shutdown(&self) {
        for j in self.inner.lock().await.values() {
            let _ = j.cancel.send(true);
        }
        let _ = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if self
                    .inner
                    .lock()
                    .await
                    .values()
                    .all(|j| j.state["status"] != "running")
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await;
    }
}
fn kill_group(pid: Option<u32>) {
    #[cfg(unix)]
    if let Some(pid) = pid
        && let Ok(pid) = i32::try_from(pid)
    {
        /* SAFETY: process_group(0) gave this child its own group. */
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    let _ = pid;
}
async fn capture(mut reader: impl AsyncRead + Unpin) -> String {
    let mut kept = vec![];
    let mut buf = [0; 8192];
    let mut truncated = false;
    loop {
        match reader.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let remaining = 65536usize.saturating_sub(kept.len());
                kept.extend_from_slice(&buf[..n.min(remaining)]);
                truncated |= n > remaining;
            }
        }
    }
    let mut text = String::from_utf8_lossy(&kept).into_owned();
    if truncated {
        text.push_str("\n[output truncated at 64 KiB]");
    }
    text
}
