use std::{collections::HashMap, fs, path::PathBuf};

use clap::Parser;

mod consent;
mod docker;
mod patient;

use consent::Consent;
use docker::Docker;
use patient::Patient;
use tracing::level_filters::LevelFilter;
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
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let stdout_log = tracing_subscriber::fmt::layer().pretty();

    let filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()?
        .add_directive("prepare_dbs=debug".parse()?);

    tracing_subscriber::registry()
        .with(stdout_log)
        .with(filter)
        .init();

    let cli = Cli::parse();
    let d = Docker::new(cli.docker_compose);

    let data_dir = cli.data_dir;
    let mut authored_dates_file = data_dir.clone();
    authored_dates_file.push("authored.json");

    let ids = cli.n.map(|n| {
        let authored_dates = fs::read_to_string(&authored_dates_file).expect("Cannot read file");
        let authored_dates: HashMap<String, String> =
            serde_json::from_str(&authored_dates).expect("Cannot parse JSON");
        authored_dates
            .keys()
            .take(n)
            .cloned()
            .collect::<Vec<String>>()
    });

    let cd_hds_url = d.cd_hds_url()?;
    let mut patients_dir = data_dir.clone();
    patients_dir.push("kds");
    let ids_clone = ids.clone();
    let patient_handle = tokio::spawn(async move {
        let patient = Patient::new(patients_dir, cd_hds_url);
        patient.upload(ids_clone).await.unwrap();
    });

    let consent_handle = tokio::spawn(async move {
        let consent = Consent::new(
            cli.consent_template,
            d.gics_url().unwrap(),
            authored_dates_file,
        )
        .unwrap();
        consent.upload(ids).await.unwrap();
    });

    patient_handle.await?;
    consent_handle.await?;

    Ok(())
}
