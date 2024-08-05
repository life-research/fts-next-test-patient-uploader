use std::{
    io,
    path::PathBuf,
    process::{Command, Output},
};

use anyhow::anyhow;
use url::Url;

// /home/trobanga/development/mii/fts-next/.github/test/compose.yaml
pub(crate) struct Docker {
    compose_file: Option<PathBuf>,
}

impl Docker {
    pub(crate) fn new(compose_file: Option<PathBuf>) -> Self {
        Self { compose_file }
    }

    pub(crate) fn port(&self, name: &str, port: u16) -> Result<Output, io::Error> {
        let mut cmd = Command::new("docker");
        cmd.arg("compose");
        if let Some(compose_file) = &self.compose_file {
            cmd.arg("-f").arg(compose_file);
        }
        cmd.arg("port").arg(name).arg(port.to_string()).output()
    }

    pub(crate) fn base_url(&self, name: &str, port: u16) -> anyhow::Result<Url> {
        let binding = self.port(name, port)?;
        if binding.status.success() {
            let url = String::from_utf8_lossy(&binding.stdout);
            let url = url.trim();
            Url::parse(&format!("http://{}/", url)).map_err(|e| anyhow!(e))
        } else {
            Err(anyhow!("Cannot determine base url for {name}"))
        }
    }
}
