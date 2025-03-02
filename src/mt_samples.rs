use std::error::Error;
use std::fs::OpenOptions;
use reqwest::Client;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Header {
    pub specialty: String,
    pub name: String,
    pub description: String,
    pub text: String,
    pub keywords: String,
    pub url: String
}


pub async fn get_parsed_page(client: Client, url: String) -> anyhow::Result<String> {

    let response = client.get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await?
        .text()
        .await?;

    Ok(response)

}

pub async fn convert_to_header(data: Vec<String>) -> anyhow::Result<Header> {

    let header_data: crate::Header = crate::Header {
        specialty: String::from(&data[1]),
        name: String::from(&data[3]),
        description: String::from(&data[5]),
        text: String::from(""),
        keywords: String::from(""),
        url: String::from("")
    };

    Ok(header_data)

}

pub async fn append_as_ndjson(header: &Header, filename: &PathBuf) -> anyhow::Result<(), Box<dyn Error>> {

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(filename)?;

    let json_string = serde_json::to_string(&header)?;
    writeln!(file, "{}", json_string)?;

    Ok(())
}


pub async fn get_node_content(sub_page: Html, selector: &Selector) -> Option<Vec<String>> {

    let mut non_headers: Vec<String> = Vec::new();

    if let Some(bold_select) = sub_page.select(selector).next() {
        for node in bold_select.children() {
            if let Some(text) = node.value().as_text() {
                let cleaned = text.trim().to_string();
                if !cleaned.is_empty() {
                    non_headers.push(cleaned);
                }
            } else if let Some(element) = node.value().as_element() {
                let text: String = node.descendants()
                    .filter_map(|n| n.value().as_text())
                    .map(|text| text.trim())
                    .filter(|text| !text.is_empty())
                    .collect();
                if !text.is_empty() {
                    non_headers.push(text);
                }
            }
        }
    }

    Some(non_headers)
}
