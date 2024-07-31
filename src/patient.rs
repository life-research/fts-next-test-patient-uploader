use std::{fs, path::PathBuf};

use glob::glob;
use reqwest::Client;
use tracing::{debug, error, instrument};
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
    pub(crate) async fn upload(&self) -> anyhow::Result<()> {
        let mut path = self.patient_dir.clone();
        path.push("**/*.json");
        let patients = path
            .to_str()
            .iter()
            .flat_map(|path| glob(path).unwrap_or_else(|_| panic!("Failed to read path {path}")))
            .filter_map(|entry| entry.ok())
            .collect::<Vec<PathBuf>>();

        for patient in patients {
            let s = self.clone();
            tokio::spawn(async move {
                debug!("Upload patient data for {}", patient.display());
                let patient_data = fs::read_to_string(patient).unwrap();
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
                            error!("Err: {e}")
                        }
                    }
                    Err(e) => {
                        error!("Err {e}");
                    }
                }
            })
            .await?;
        }
        Ok(())
    }
}
