// src/pdf_download.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use log::{info, error};
use tokio::task;
use anyhow::{Result, Context};
use indicatif::ProgressBar;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Member {
    pub prefix: Option<String>,
    pub last: String,
    pub first: String,
    pub suffix: Option<String>,
    #[serde(rename = "FilingType")]
    pub filing_type: String,
    #[serde(rename = "StateDst")]
    pub state_dst: String,
    pub year: u16,
    pub filing_date: String,
    #[serde(rename = "DocID")]
    pub document_id: u64,
}

pub async fn get_pdf_reports() -> Result<()> {
    // Initialize the HTTP client
    let client = Client::new();
    let members: Vec<Member> = load_members_from_yaml("data/documents.yml")?;

    download_pdfs(&client, members).await
}

fn load_members_from_yaml(file_path: &str) -> Result<Vec<Member>> {
    let yaml_str = std::fs::read_to_string(file_path)?;
    let members: Vec<Member> = serde_yaml::from_str(&yaml_str)?;
    Ok(members)
}

pub async fn download_pdfs(client: &Client, members: Vec<Member>) -> Result<()> {
    let mut handles = vec![];

    for member in members {
        let client = client.clone();
        handles.push(task::spawn(async move {
            let url = format!(
                "https://disclosures-clerk.house.gov/public_disc/financial-pdfs/{}/{}.pdf",
                member.year, member.document_id
            );
            let response = client.get(&url).send().await?.bytes().await?;

            // Ensure the state_dst is at least 4 characters long
            if member.state_dst.len() < 4 {
                return Err(anyhow::anyhow!("Invalid StateDst: {:?}", member.state_dst));
            }

            let state_code = &member.state_dst[..2];
            let district_number = &member.state_dst[2..];
            let district_number = format!("{:0>2}", district_number); // Adding leading zeros

            let directory_path = format!(
                "data/raw/reports/{}/{}/{}",
                state_code, district_number, member.year
            );
            fs::create_dir_all(&directory_path)
                .context(format!("Failed to create directory {}", directory_path))?;

            let file_path = format!("{}/{}.pdf", directory_path, member.document_id);
            let mut file = File::create(&file_path)
                .context(format!("Failed to create file {}", file_path))?;
            file.write_all(&response)
                .context(format!("Failed to write to file {}", file_path))?;

            info!("Downloaded and saved PDF for {}{} from {}", state_code, district_number, member.year);

            Ok::<(), anyhow::Error>(())
        }));
    }

    for handle in handles {
        handle.await??;
    }
    Ok(())
}