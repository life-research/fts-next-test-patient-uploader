use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{atomic::AtomicU32, Arc},
};

use reqwest::Client;
use tracing::{error, instrument, trace};
use url::Url;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub(crate) struct Consent {
    template: String,
    client: Client,
    gics_url: Url,
    authored_dates: PathBuf,
}

impl Consent {
    pub(crate) fn new(
        template: PathBuf,
        gics_url: Url,
        authored_dates: PathBuf,
    ) -> anyhow::Result<Self> {
        let template = fs::read_to_string(template)?;
        let client = Client::new();

        let gics_url = gics_url.join("ttp-fhir/fhir/gics/$addConsent")?;

        Ok(Self {
            template,
            client,
            gics_url,
            authored_dates,
        })
    }

    #[instrument]
    pub(crate) async fn upload(&self, ids: Option<Vec<String>>) -> anyhow::Result<Arc<AtomicU32>> {
        let authored_dates = ids.map_or_else(
            || {
                let authored_dates =
                    fs::read_to_string(&self.authored_dates).expect("Cannot read authored.json");
                let authored_dates: HashMap<String, String> =
                    serde_json::from_str(&authored_dates).expect("Cannot parse JSON");
                authored_dates
            },
            |ids| {
                let authored_dates =
                    fs::read_to_string(&self.authored_dates).expect("Cannot read authored.json");
                let authored_dates: HashMap<String, String> =
                    serde_json::from_str(&authored_dates).expect("Cannot parse JSON");
                authored_dates
                    .into_iter()
                    .filter(|(k, _)| ids.contains(k))
                    .collect::<HashMap<String, String>>()
            },
        );

        let cnt = Arc::new(AtomicU32::new(0));

        for (id, authored) in authored_dates.iter() {
            let consent = self.clone();
            let id = id.clone();
            let authored = authored.clone();
            let cnt = cnt.clone();
            tokio::spawn(async move {
                trace!("Upload consent for {id}");
                let client = consent.client;
                let template = consent.template;
                let template = template.replace("$PATIENT_ID", &id);
                let template =
                    template.replace("$QUESTIONNAIRE_RESPONSE_UUID", &Uuid::new_v4().to_string());
                let template =
                    template.replace("$RESEARCH_STUDY_UUID", &Uuid::new_v4().to_string());

                let template = template.replace("$AUTHORED", &authored);

                let res = client
                    .post(consent.gics_url.to_string())
                    .header("Content-Type", "application/fhir+json")
                    .body(template)
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

    pub(crate) fn check_transfer_successful(&self) -> anyhow::Result<()> {
        // let url = self.gics_url.;
        Ok(())
    }
}
