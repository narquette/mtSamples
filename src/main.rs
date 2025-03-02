use reqwest::{Client};
use scraper::{Html, Selector};
use anyhow::Result;
//use scraper::node::Text;
use tokio;
use serde::{Serialize, Deserialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
struct Header {
    specialty: String,
    name: String,
    description: String,
    text: String,
    keywords: String,
    url: String
}


async fn get_parsed_page(client: Client, url: String) -> Result<String> {

    let response = client.get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .send()
        .await?
        .text()
        .await?;

    Ok(response)

}

async fn convert_to_header(data: Vec<String>) -> Result<Header> {

    let header_data: Header = Header {
        specialty: String::from(&data[1]),
        name: String::from(&data[3]),
        description: String::from(&data[5]),
        text: String::from(""),
        keywords: String::from(""),
        url: String::from("")
    };

    Ok(header_data)

}

async fn append_as_ndjson(header: &Header, filename: &str) -> Result<(), Box<dyn Error>> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(filename)?;

    let json_string = serde_json::to_string(&header)?;
    writeln!(file, "{}", json_string)?;

    Ok(())
}


async fn get_node_content(sub_page: Html, selector: &Selector) -> Option<Vec<String>> {

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


#[tokio::main]
async fn main() -> Result<()> {

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

                                append_as_ndjson(&cur_header, "header_output.jsonl").await.expect("Failed to Add Header");
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