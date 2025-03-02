mod mt_samples;

use reqwest::{Client};
use scraper::{Html, Selector};
use anyhow::Result;
use tokio;
use serde::{Serialize, Deserialize};
use std::io::Write;
use std::error::Error;
use std::fs::{create_dir_all};
use std::path::{PathBuf};
use mt_samples::{get_node_content, get_parsed_page, convert_to_header, Header, append_as_ndjson};
use dirs::home_dir;

#[tokio::main]
async fn main() -> Result<()> {

    // set home directory
    let home_dir = home_dir();

    // Set out_output
    let out_path: PathBuf = home_dir.unwrap().join("data").join("mtSamples");

    // Create an output directory
    create_dir_all(&out_path).expect("Failed to create directory");

    // Create a new HTTP client
    let client = Client::new();

    // Define the URL to scrape
    let url = "https://mtsamples.com";

    // Make the HTTP request and get the response body
    let response = get_parsed_page(client.clone(), url.to_string()).await?;

    // Parse the HTML document
    let document = Html::parse_document(&response);

    // Create selectors for different types of links
    let all_links_selector = Selector::parse("a").unwrap();

    // Extract all links
    println!("\nGetting Link Data");

    // let mut sites= Vec::new();
    // let all_data: Vec<Header> = Vec::new();
    for link in document.select(&all_links_selector) {
        // Get the href attribute and link text
        if let Some(href) = link.value().attr("href") {

            // Optional: Check if it's an absolute or relative URL
            if href.starts_with("/site/pages/browse.asp?Type") {
                let url_str = String::from(url);
                let cur_site = url_str + href;

                let sub_response = get_parsed_page(client.clone(), cur_site.to_string()).await?;
                let sub_page: Html = Html::parse_document(&sub_response);

                for sub_link in sub_page.select(&all_links_selector) {
                    if let Some(sub_href) = sub_link.value().attr("href") {

                        if sub_href.contains("Sample") {

                            let cur_site = String::from(url) + sub_href;
                            // sites.push(cur_site.clone());
                            let page_response = get_parsed_page(client.clone(), cur_site.clone()).await?;
                            let sub_page: Html = Html::parse_document(&page_response);

                            // build vectors to store header and non-header information
                            let mut headers: Vec<String> = Vec::new();
                            let mut non_headers: Vec<String> = Vec::new();

                            // get header content
                            for level in 1..=2 {
                                let head_selector = Selector::parse(&format!("h{}", level)).unwrap();
                                let cur_results = get_node_content(sub_page.clone(), &head_selector).await.unwrap();
                                headers.extend(cur_results);
                            }

                            // get non header content
                            let non_head_selector = Selector::parse("div.hilightBold").unwrap();
                            let cur_results = get_node_content(sub_page.clone(), &non_head_selector).await.unwrap();
                            non_headers.extend(cur_results);

                            // remove information from headers as it is a duplicate
                            if headers.len() == 7 {
                                headers.remove(6); //remove standard statement
                                let mut cur_header: Header = convert_to_header(headers.clone()).await?;

                                non_headers.remove(0); //remove header information
                                let non_header_clean: String = non_headers.join(" ");
                                //let mut non_header_items: Vec<String> = non_header_clean.collect();
                                cur_header.text = String::from(non_header_clean.split("/").next().unwrap());
                                cur_header.keywords = String::from(non_header_clean.split("/").last().unwrap().trim());
                                cur_header.url = String::from(cur_site);

                                append_as_ndjson(&cur_header, &out_path.join("header_output.jsonl")).await.expect("Failed to Add Header");
                            } else {
                                println!("\nHeaders len: {}\n Current Link: {}", headers.len(), cur_site);
                            }

                        }
                    }

                }
            }
        }
    }

    Ok(())
}