use std::{
    fs,
    path::PathBuf,
    sync::{atomic::AtomicU32, Arc},
};

use reqwest::Client;
use tracing::{error, instrument, trace};
use url::Url;

#[derive(Debug, Clone)]
pub(crate) struct Patient {
    patient_dir: PathBuf,
    client: Client,
    hds_url: Url,
}

impl Patient {
    pub(crate) fn new(patient_dir: PathBuf, hds_url: Url) -> Self {
        let client = Client::new();
        Self {
            patient_dir,
            client,
            hds_url,
        }
    }

    #[instrument]
    pub(crate) async fn upload(&self, ids: &Vec<String>) -> anyhow::Result<Arc<AtomicU32>> {
        let cnt = Arc::new(AtomicU32::new(0));
        for id in ids {
            let s = self.clone();
            let cnt = cnt.clone();
            let id = id.clone();
            tokio::spawn(async move {
                let mut patient = s.patient_dir;
                patient.push(format!("{id}.json"));
                trace!("Upload patient data for {}", patient.display());
                let patient_data = fs::read_to_string(patient).unwrap();
                trace!("Data len {}", patient_data.len());
                let res = s
                    .client
                    .post(s.hds_url.to_string())
                    .header("Content-Type", "application/fhir+json")
                    .body(patient_data)
                    .send()
                    .await;

                match res {
                    Ok(res) => {
                        if let Err(e) = res.text().await {
                            error!("Err: {e}");
                        } else {
                            cnt.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    Err(e) => {
                        error!("Err {e}");
                    }
                }
            })
            .await?;
        }
        Ok(cnt)
    }
}
