use std::{
    fs,
    path::PathBuf,
    sync::{atomic::AtomicU32, Arc},
};

use glob::glob;
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
    pub(crate) async fn upload(&self, ids: Option<Vec<String>>) -> anyhow::Result<Arc<AtomicU32>> {
        let mut path = self.patient_dir.clone();
        path.push("*.json");

        let patients = ids.map_or_else(
            || {
                path.to_str()
                    .iter()
                    .flat_map(|path| {
                        glob(path).unwrap_or_else(|_| panic!("Failed to read path {path}"))
                    })
                    .filter_map(Result::ok)
                    .collect::<Vec<PathBuf>>()
            },
            |ids| {
                path.to_str()
                    .iter()
                    .flat_map(|path| {
                        glob(path).unwrap_or_else(|_| panic!("Failed to read path {path}"))
                    })
                    .filter_map(Result::ok)
                    .filter(|p| ids.contains(&p.file_stem().unwrap().to_str().unwrap().to_string()))
                    .collect::<Vec<PathBuf>>()
            },
        );

        let cnt = Arc::new(AtomicU32::new(0));
        for patient in patients {
            let s = self.clone();
            let cnt = cnt.clone();
            tokio::spawn(async move {
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
