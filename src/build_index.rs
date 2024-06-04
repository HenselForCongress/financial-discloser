// src/build_index.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Cursor, Write};
use std::path::Path;
use zip::read::ZipArchive;
use anyhow::{Result, Context};
use tracing::{info, error};

const BASE_URL: &str = "https://disclosures-clerk.house.gov/public_disc/financial-pdfs/";

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

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct FinancialDisclosure {
    pub member: Vec<Member>,
}

async fn download_and_extract(year: u16) -> Result<String> {
    let url = format!("{}{}FD.zip", BASE_URL, year);
    let response = reqwest::get(&url).await?.bytes().await?;

    let reader = Cursor::new(response);
    let mut zip = ZipArchive::new(reader)?;

    let xml_filename = format!("data/raw/indexes/{}FD.xml", year);
    fs::create_dir_all(Path::new(&xml_filename).parent().unwrap())?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        if file.name().ends_with(".xml") {
            let mut out_file = File::create(&xml_filename)?;
            io::copy(&mut file, &mut out_file)?;
            return Ok(xml_filename);
        }
    }
    Err(anyhow::anyhow!("XML file not found in the zip archive"))
}

fn parse_xml(file_path: &str) -> Result<Vec<Member>> {
    let xml_content = fs::read_to_string(file_path)?;
    let disclosure: FinancialDisclosure = quick_xml::de::from_str(&xml_content)?;
    Ok(disclosure.member)
}

fn save_to_yaml(members: &Vec<Member>, output_file: &str) -> Result<()> {
    let yaml_content = serde_yaml::to_string(members)?;
    fs::create_dir_all(Path::new(output_file).parent().unwrap())?;
    let mut file = File::create(output_file)?;
    file.write_all(yaml_content.as_bytes())?;
    Ok(())
}

pub async fn get_updated_index() -> Result<()> {
    let mut all_members = Vec::new();
    for year in 2022..=2024 {
        match download_and_extract(year).await {
            Ok(xml_filepath) => {
                info!("Downloaded and saved XML for year: {}", year);
                match parse_xml(&xml_filepath) {
                    Ok(members) => {
                        info!("Parsed XML for year: {}", year);
                        all_members.extend(members);
                    }
                    Err(e) => error!("Failed to parse XML: {}", e),
                }
            }
            Err(e) => error!("Failed to download or extract XML for year {}: {}", year, e),
        }
    }

    save_to_yaml(&all_members, "data/documents.yml")?;
    info!("YAML file saved to data/documents.yml");

    Ok(())
}
