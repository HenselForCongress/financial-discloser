// src/pdf_download.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use log::{info, error, warn, debug};
use anyhow::{Result, Context};
use indicatif::{ProgressBar, ProgressStyle};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use serde_yaml;

#[derive(Debug, Serialize, Deserialize, Default)]
struct DownloadReport {
    successful: Vec<u64>,
    failed: Vec<u64>,
    blocked: Vec<u64>,
    not_found: Vec<u64>,
}

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
    info!("Starting to get PDF reports.");
    println!("Starting to get PDF reports."); // Debug statement
    let client = Client::new();
    let members: Vec<Member> = load_members_from_yaml("data/documents.yml")?;

    let (to_download, already_downloaded) = filter_not_downloaded_reports(&members);

    info!("Reports needing download: {}", to_download.len());
    println!("Reports needing download: {}", to_download.len()); // Debug statement

    info!("Reports already downloaded: {}", already_downloaded.len());
    println!("Reports already downloaded: {}", already_downloaded.len()); // Debug statement

    Command::new("scripts/connect_vpn.sh").output().expect("Failed to execute script");

    download_pdfs(&client, to_download).await
}

fn load_members_from_yaml(file_path: &str) -> Result<Vec<Member>> {
    info!("Loading members from YAML file: {}", file_path);
    let yaml_str = std::fs::read_to_string(file_path)?;
    let members: Vec<Member> = serde_yaml::from_str(&yaml_str)?;
    info!("Loaded {} members from YAML file", members.len());
    Ok(members)
}

fn filter_not_downloaded_reports(members: &Vec<Member>) -> (Vec<Member>, Vec<Member>) {
    info!("Filtering reports that need to be downloaded");
    let mut to_download = Vec::new();
    let mut already_downloaded = Vec::new();

    for member in members {
        if member.state_dst.len() < 4 {
            warn!("Invalid StateDst for member: {:?}", member);
            continue;
        }

        let state_code = &member.state_dst[..2];
        let district_number: u8 = match member.state_dst[2..].parse() {
            Ok(num) => num,
            Err(_) => {
                warn!("Invalid StateDst for member: {:?}", member);
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

    info!("{} reports to download. {} reports already downloaded.", to_download.len(), already_downloaded.len());
    (to_download, already_downloaded)
}

pub async fn download_pdfs(client: &Client, members: Vec<Member>) -> Result<()> {
    info!("Starting to download PDFs");
    println!("Starting to download PDFs"); // Debug statement

    let pb = ProgressBar::new(members.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {msg}")
        .unwrap()
        .progress_chars("#>-"));

    let mut report = DownloadReport::default();

    for member in members {
        let mut attempts = 0;

        loop {
            match attempt_download(client, &member).await {
                Ok(_) => {
                    info!("Successfully downloaded PDF for document id: {}", member.document_id);
                    println!("Successfully downloaded PDF for document id: {}", member.document_id); // Debug statement
                    pb.inc(1);
                    report.successful.push(member.document_id);
                    break;
                },
                Err(err) if err == "404" => {
                    info!("Document id {} not found (404)", member.document_id);
                    println!("Document id {} not found (404)", member.document_id); // Debug statement
                    pb.inc(1);
                    report.not_found.push(member.document_id);
                    break;
                },
                Err(err) if attempts < 3 => {
                    attempts += 1;
                    error!("Failed to download PDF for document id {}: {}. Retrying...", member.document_id, err);
                    println!("Failed to download PDF for document id {}: {}. Retrying...", member.document_id, err); // Debug statement

                    // Log VPN rotation
                    info!("Rotating VPN server");
                    println!("Rotating VPN server"); // Debug statement

                    // Execute the VPN rotation script
                    Command::new("scripts/rotate_vpn.sh")
                        .output().expect("Failed to execute script");

                    // Wait before retrying
                    sleep(Duration::from_secs(5));
                },
                Err(err) => {
                    error!("Failed to download PDF for document id {} after multiple attempts: {}", member.document_id, err);
                    println!("Failed to download PDF for document id {} after multiple attempts: {}", member.document_id, err); // Debug statement
                    pb.inc(1);
                    report.failed.push(member.document_id);
                    break;
                }
            }
        }
    }

    pb.finish_with_message("Download complete.");
    info!("Finished downloading PDFs");
    println!("Finished downloading PDFs"); // Debug statement

    // Save the report to a YAML file
    let report_content = serde_yaml::to_string(&report).expect("Failed to serialize report");
    fs::write("data/report.yml", report_content).expect("Failed to write report");

    Ok(())
}

async fn attempt_download(client: &Client, member: &Member) -> Result<(), String> {
    // Determine the URL based on the FilingType
    let url_base = if member.filing_type == "P" {
        "https://disclosures-clerk.house.gov/public_disc/ptr-pdfs"
    } else {
        "https://disclosures-clerk.house.gov/public_disc/financial-pdfs"
    };

    let url = format!("{}/{}/{}.pdf", url_base, member.year, member.document_id);
    info!("Attempting to download PDF for document id: {} from URL: {}", member.document_id, url);
    println!("Attempting to download PDF for document id: {} from URL: {}", member.document_id, url);

    let response = client.get(&url).send().await.map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let bytes = response.bytes().await.map_err(|e| e.to_string())?;

        let state_code = &member.state_dst[..2];
        let district_number: u8 = member.state_dst[2..].parse().unwrap_or(0);
        let district_number = format!("{:0>2}", district_number);

        let directory_path = format!(
            "data/raw/reports/{}/{}/{}/",
            state_code, district_number, member.year
        );
        fs::create_dir_all(&directory_path).map_err(|e| e.to_string())?;

        let file_path = format!("{}/{}.pdf", directory_path, member.document_id);
        let mut file = File::create(&file_path).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;

        info!("Downloaded and saved PDF for {}{} from {}", state_code, district_number, member.year);
        println!("Downloaded and saved PDF for {}{} from {}", state_code, district_number, member.year);
        Ok(())
    } else {
        let status = response.status();
        error!("HTTP error when attempting to download PDF for document id {}: {}", member.document_id, status);
        println!("HTTP error when attempting to download PDF for document id {}: {}", member.document_id, status);
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err("404".to_string());
        } else {
            return Err(format!("HTTP error: {}", status));
        }
    }
}
