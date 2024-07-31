use std::{collections::HashMap, fs, path::PathBuf};

use reqwest::Client;
use url::Url;
use uuid::Uuid;

#[derive(Clone)]
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
        println!("gICS URL: {gics_url}");

        Ok(Self {
            template,
            client,
            gics_url,
            authored_dates,
        })
    }

    pub(crate) async fn upload(&self) -> anyhow::Result<()> {
        let authored_dates = fs::read_to_string(&self.authored_dates)?;
        let authored_dates: HashMap<String, String> = serde_json::from_str(&authored_dates)?;
        for (id, authored) in authored_dates.iter() {
            let consent = self.clone();
            let id = id.clone();
            let authored = authored.clone();
            tokio::spawn(async move {
                println!("Send Consent: {id}");
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
                            println!("Err: {e}")
                        }
                    }
                    Err(e) => {
                        println!("Err {e}");
                    }
                }
            })
            .await?;
        }

        Ok(())
    }
}
