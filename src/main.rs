use std::path::PathBuf;

use clap::Parser;

mod consent;
mod docker;
mod patient;

use consent::Consent;
use docker::Docker;
use patient::Patient;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The directory with the patients' JSON files
    #[arg(short, long, value_name = "PATIENTS")]
    patients_dir: PathBuf,

    /// The path to the docker compose file
    #[arg(short = 'd', long, value_name = "COMPOSE")]
    docker_compose: Option<PathBuf>,

    /// The path to the consent template file
    #[arg(short, long, value_name = "CONSENT")]
    consent_template: PathBuf,

    /// The path to the authored dates file
    #[arg(short, long, value_name = "AUTHORED")]
    authored_dates: PathBuf,

    /// The number of patients to upload
    #[arg(short, long, value_name = "N")]
    n: Option<usize>,
}

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let patients_dir = cli.patients_dir;

    let d = Docker::new(cli.docker_compose);

    let cd_hds_url = d.cd_hds_url()?;
    let patient_handle = tokio::spawn(async move {
        let patient = Patient::new(patients_dir, cd_hds_url);
        patient.upload().await.unwrap();
    });

    let consent_handle = tokio::spawn(async move {
        let consent = Consent::new(
            cli.consent_template,
            d.gics_url().unwrap(),
            cli.authored_dates,
        )
        .unwrap();
        consent.upload().await.unwrap();
    });

    patient_handle.await?;
    consent_handle.await?;

    Ok(())
}
