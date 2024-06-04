// src/pdf_download.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use log::{info, error};
use tokio::task;
use anyhow::{Result, Context};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Debug, Deserialize, Serialize, Clone)]
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
    let client = Client::new();
    let members: Vec<Member> = load_members_from_yaml("data/documents.yml")?;

    let (to_download, already_downloaded) = filter_not_downloaded_reports(&members);

    info!("Reports needing download: {}", to_download.len());
    info!("Reports already downloaded: {}", already_downloaded.len());

    download_pdfs(&client, to_download).await
}

fn load_members_from_yaml(file_path: &str) -> Result<Vec<Member>> {
    let yaml_str = std::fs::read_to_string(file_path)?;
    let members: Vec<Member> = serde_yaml::from_str(&yaml_str)?;
    Ok(members)
}

fn filter_not_downloaded_reports(members: &Vec<Member>) -> (Vec<Member>, Vec<Member>) {
    let mut to_download = Vec::new();
    let mut already_downloaded = Vec::new();

    for member in members {
        if member.state_dst.len() < 4 {
            error!("Invalid StateDst for member: {:?}", member);
            continue;
        }

        let state_code = &member.state_dst[..2];
        let district_number: u8 = match member.state_dst[2..].parse() {
            Ok(num) => num,
            Err(_) => {
                error!("Invalid StateDst for member: {:?}", member);
                continue;
            }
        };
        let district_number = format!("{:0>2}", district_number); // Adding leading zeros

        let file_path = format!(
            "data/raw/reports/{}/{}/{}/{}.pdf",
            state_code, district_number, member.year, member.document_id
        );
        if !fs::metadata(&file_path).is_ok() {
            to_download.push(member.clone());
        } else {
            already_downloaded.push(member.clone());
        }
    }

    (to_download, already_downloaded)
}

pub async fn download_pdfs(client: &Client, members: Vec<Member>) -> Result<()> {
    let pb = ProgressBar::new(members.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
        .unwrap()
        .progress_chars("#>-"));

    let mut handles = vec![];

    for member in members {
        let client = client.clone();
        let pb = pb.clone();
        handles.push(task::spawn(async move {
            let url = format!(
                "https://disclosures-clerk.house.gov/public_disc/financial-pdfs/{}/{}.pdf",
                member.year, member.document_id
            );
            let response = client.get(&url).send().await?;

            if response.status().is_success() {
                let bytes = response.bytes().await?;

                if member.state_dst.len() < 4 {
                    error!("Invalid StateDst for member: {:?}", member);
                    return Ok(());
                }

                let state_code = &member.state_dst[..2];
                let district_number: u8 = match member.state_dst[2..].parse() {
                    Ok(num) => num,
                    Err(_) => {
                        error!("Invalid StateDst for member: {:?}", member);
                        return Ok(());
                    }
                };
                let district_number = format!("{:0>2}", district_number); // Adding leading zeros

                let directory_path = format!(
                    "data/raw/reports/{}/{}/{}/",
                    state_code, district_number, member.year
                );
                fs::create_dir_all(&directory_path)
                    .context(format!("Failed to create directory {}", directory_path))?;

                let file_path = format!("{}/{}.pdf", directory_path, member.document_id);
                let mut file = File::create(&file_path)
                    .context(format!("Failed to create file {}", file_path))?;
                file.write_all(&bytes)
                    .context(format!("Failed to write to file {}", file_path))?;

                info!("Downloaded and saved PDF for {}{} from {}", state_code, district_number, member.year);
            } else {
                error!("Failed to download PDF for document id: {}", member.document_id);
            }

            pb.inc(1);
            Ok::<(), anyhow::Error>(())
        }));
    }

    for handle in handles {
        if let Err(e) = handle.await? {
            error!("Error downloading PDF: {}", e);
        }
    }

    pb.finish_with_message("download complete.");
    Ok(())
}
