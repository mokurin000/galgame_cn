use std::io::{BufRead, BufReader};

use reqwest::blocking::Client;
use reqwest::Proxy;

use regex::Regex;

const HTTPS_PROXY: &str = "http://192.168.1.4:7890";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sites_list = std::fs::File::open("../sites.txt")?;
    let reader = BufReader::new(sites_list);
    let mut sites = reader.lines();

    let client = Client::builder()
        .proxy(Proxy::https(HTTPS_PROXY)?)
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/103.0.0.0 Safari/537.36")
        .build()?;

    let regex = Regex::new("<title[^>]*>(.*)</title>")?;
    while let Some(Ok(site)) = sites.next() {
        if site.is_empty() {
            break;
        } // exit on blank line

        let resp = client.get(&site).send()?.text()?;
        let title = regex.captures(&resp).unwrap().get(1).unwrap().as_str();
        println!("[{title}]({site})");
    }

    Ok(())
}
