use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use ratkara_core::content::PROJECTS;

pub type Stats = Arc<RwLock<Vec<Option<u32>>>>;

pub fn spawn_refresh() -> Stats {
    let stats: Stats = Arc::new(RwLock::new(vec![None; PROJECTS.len()]));
    let shared = stats.clone();
    tokio::spawn(async move {
        loop {
            for (idx, project) in PROJECTS.iter().enumerate() {
                let url = project.api_url();
                let count = tokio::task::spawn_blocking(move || {
                    ureq::get(&url)
                        .header("User-Agent", "ratkaradotdev")
                        .call()
                        .ok()?
                        .body_mut()
                        .read_to_string()
                        .ok()
                        .as_deref()
                        .and_then(|json| project.parse_stat(json))
                })
                .await
                .ok()
                .flatten();
                if let (Some(count), Ok(mut stats)) = (count, shared.write()) {
                    stats[idx] = Some(count);
                }
            }
            tokio::time::sleep(Duration::from_hours(1)).await;
        }
    });
    stats
}
