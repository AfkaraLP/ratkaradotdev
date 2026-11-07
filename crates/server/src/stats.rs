use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use ratkara_core::content::{GITHUB, PROJECTS, Project};

pub type Stats = Arc<RwLock<Vec<Option<u32>>>>;

pub fn spawn_refresh(github_token: Option<String>) -> Stats {
    let stats: Stats = Arc::new(RwLock::new(vec![None; PROJECTS.len()]));
    let shared = stats.clone();
    tokio::spawn(async move {
        loop {
            for (idx, project) in PROJECTS.iter().enumerate() {
                let auth = github_token
                    .as_ref()
                    .filter(|_| project.host.api == GITHUB.api)
                    .map(|token| format!("Bearer {token}"));
                let count = tokio::task::spawn_blocking(move || fetch(project, auth))
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

fn fetch(project: &Project, auth: Option<String>) -> Option<u32> {
    let request = ureq::get(project.api_url()).header("User-Agent", "ratkaradotdev");
    let request = match auth {
        Some(auth) => request.header("Authorization", auth),
        None => request,
    };
    let body = request.call().ok()?.body_mut().read_to_string().ok()?;
    project.parse_stat(&body)
}
