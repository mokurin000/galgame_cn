use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Duration;

use playwright::Playwright;
use std::sync::Arc;
use tokio::task::JoinSet;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Args {
        sites,
        output,
        proxy,
    } = Args::parse();

    let current_list = unsafe { String::from_utf8_unchecked(fs::read(&output)?) };
    let sites_list = unsafe { String::from_utf8_unchecked(fs::read(sites)?) };
    let output = OpenOptions::new()
        .create(false)
        .append(true)
        .read(false)
        .open(output)?;
    let mut out = BufWriter::new(output);

    let pw = Playwright::initialize().await?;
    pw.install_chromium()?;
    let chromium = pw.chromium();
    let mut launcher = chromium.launcher().headless(true);

    if let Some(proxy) = proxy {
        launcher = launcher.proxy(ProxySettings {
            server: proxy,
            bypass: None,
            username: None,
            password: None,
        });
    }

    let browser = launcher.launch().await?;
    let context = Arc::new(browser.context_builder().user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36 GLS/100.10.9939.100").build().await?);

    // bypass CF for ryuugames
    context.add_cookies(&[Cookie::with_domain_path("cf_clearance", "CqCW39e7Dqzq4e74ac4NBDP_TkjJuvM6TtGCyEHqtQE-1723046672-1.0.1.1-_1Dj47lZPpDpiW6Iw6zb_lg3ZrmKgkJpxrRcxwhKWsXRtmHFy.YSBcCOYupK.I.ZSZ7tmJbfU729PlTb6K0NpQ", ".ryuugames.com", "/")]).await?;

    let mut task_set: JoinSet<anyhow::Result<Option<String>>> = JoinSet::new();

    if !std::fs::read_dir("res").is_ok() {
        std::fs::create_dir("res")?;
    }

    for site in sites_list
        .lines()
        .filter(|&site| !site.is_empty())
        .filter(|&site| !current_list.contains(site))
        .map(|s| s.to_owned())
    {
        let context = context.clone();
        task_set.spawn(async move {
            let page = context.new_page().await?;

            let Ok(Some(_)) = page
                .goto_builder(&site)
                .wait_until(DocumentLoadState::Load)
                .goto()
                .await
            else {
                return Ok(None);
            };

            let _site = site.clone();
            let screenshot_out = tokio::task::spawn_blocking(move || {
                format!(
                    "res/{}.webp",
                    hex_simd::encode_to_string(md5::compute(&_site).0, hex_simd::AsciiCase::Lower)
                )
            })
            .await?;
            println!("screenshot path for {site}: {screenshot_out}");

            tokio::time::sleep(Duration::from_secs(10)).await;

            let title = page.title().await?;

            page.screenshot_builder()
                .path(screenshot_out.clone().into())
                .screenshot()
                .await?;

            let md_line = tokio::task::spawn_blocking(move || {
                format!("[![{title}]({screenshot_out})]({site})\n\n")
            })
            .await?;
            Ok(Some(md_line))
        });
    }

    while let Some(Ok(re)) = task_set.join_next().await {
        if let Ok(Some(line)) = re {
            out.write(line.as_bytes())?;
        }
    }

    Ok(())
}

use clap::Parser;
use clap::ValueHint;
use playwright::api::{Cookie, DocumentLoadState, ProxySettings};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// plain text file, contains sites to generate
    #[clap(short, long, value_parser, value_hint = ValueHint::FilePath)]
    sites: PathBuf,

    /// result file, should exists
    #[clap(short, long, value_parser, value_hint = ValueHint::FilePath)]
    output: PathBuf,

    /// [optional] proxy to set, for example http://127.0.0.1:8080
    #[clap(short, long, value_parser)]
    proxy: Option<String>,
}
