use std::{collections::HashMap, fs, path::PathBuf};

use clap::Parser;

mod consent;
mod docker;
mod patient;

use consent::Consent;
use docker::Docker;
use patient::Patient;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The directory conaining authored.json & /kds with patients' JSON files
    #[arg(short = 'i', long, value_name = "DATA_DIR")]
    data_dir: PathBuf,

    /// The path to the docker compose file
    #[arg(short = 'd', long, value_name = "COMPOSE")]
    docker_compose: Option<PathBuf>,

    /// The path to the consent template file
    #[arg(short, long, value_name = "CONSENT")]
    consent_template: PathBuf,

    /// The number of patients to upload
    #[arg(short, long, value_name = "N")]
    n: Option<usize>,

    /// Upload specific IDs, this options overrules the `n` paramter
    #[arg(long, value_name = "IDS", use_value_delimiter = true)]
    ids: Option<Vec<String>>,
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let stdout_log = tracing_subscriber::fmt::layer().pretty();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()?
        .add_directive("upload_test_patients=debug".parse()?);

    tracing_subscriber::registry()
        .with(stdout_log)
        .with(filter)
        .init();

    let cli = Cli::parse();
    let d = Docker::new(cli.docker_compose);

    let data_dir = cli.data_dir;
    let mut authored_dates_file = data_dir.clone();
    authored_dates_file.push("authored.json");

    let ids = cli.ids.unwrap_or_else(|| {
        cli.n.map_or_else(
            || {
                let authored_dates =
                    fs::read_to_string(&authored_dates_file).expect("Cannot read file");
                let authored_dates: HashMap<String, String> =
                    serde_json::from_str(&authored_dates).expect("Cannot parse JSON");
                let mut authored_dates = authored_dates.keys().cloned().collect::<Vec<String>>();
                authored_dates.sort();
                authored_dates.into_iter().collect::<Vec<String>>()
            },
            |n| {
                let authored_dates =
                    fs::read_to_string(&authored_dates_file).expect("Cannot read file");
                let authored_dates: HashMap<String, String> =
                    serde_json::from_str(&authored_dates).expect("Cannot parse JSON");
                let mut authored_dates = authored_dates.keys().cloned().collect::<Vec<String>>();
                authored_dates.sort();
                authored_dates.into_iter().take(n).collect::<Vec<String>>()
            },
        )
    });

    let cd_hds_url = d.base_url("cd-hds", 8080)?.join("fhir")?;
    info!("CD HDS URL: {cd_hds_url}");
    let mut patients_dir = data_dir.clone();
    patients_dir.push("kds");
    let ids_clone = ids.clone();
    let patient_handle = tokio::spawn(async move {
        let patient = Patient::new(patients_dir, cd_hds_url);
        let cnt = patient.upload(&ids_clone).await.unwrap();
        info!("Transferred {:?} patients", cnt);
    });

    let gics_url = d.base_url("gics", 8080).unwrap();
    info!("gICS-web : {}", gics_url.clone().join("gics-web")?);
    let consent_handle = tokio::spawn(async move {
        let consent = Consent::new(cli.consent_template, gics_url, authored_dates_file).unwrap();
        let cnt = consent.upload(&ids).await.unwrap();
        info!("Transferred {:?} consents", cnt);

        consent.check_transfer_successful(ids).await.unwrap();
    });

    patient_handle.await?;
    consent_handle.await?;

    Ok(())
}
