use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::Duration;

use playwright::Playwright;
use std::sync::Arc;
use tokio::task::JoinSet;

const MAX_RETRY: i32 = 5;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let Args {
        sites,
        output,
        proxy,
    } = Args::parse();

    let current_list = unsafe { String::from_utf8_unchecked(fs::read(&output)?) };
    let sites_list = unsafe { String::from_utf8_unchecked(fs::read(sites)?) };
    let output = OpenOptions::new().append(true).read(false).open(output)?;
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
    let context = Arc::new(browser.context_builder().build().await?);

    let mut task_set: JoinSet<anyhow::Result<Option<String>>> = JoinSet::new();

	std::fs::create_dir("res")?;
	
    for site in sites_list
        .lines()
        .filter(|&site| !site.is_empty())
        .filter(|&site| !current_list.contains(site))
        .map(|s| s.to_owned())
    {
        let context = context.clone();
        task_set.spawn(async move {
            let page = context.new_page().await?;

            let mut load_ok = false;
            for _ in 0..MAX_RETRY {
                if let Ok(Some(_)) = page.goto_builder(&site).goto().await {
                    load_ok = true;
                    break;
                }
            }

            if !load_ok {
                return Ok(None);
            }

            tokio::time::sleep(Duration::from_secs(10)).await;

            let screenshot = format!(
                "res/{}.webp",
                hex_simd::encode_to_string(md5::compute(&site).0, hex_simd::AsciiCase::Lower)
            );

            let title = page.title().await?;
            let out = Some(format!("[![{title}]({screenshot})]({site})\n\n"));

            page.screenshot_builder()
                .path(screenshot.into())
                .screenshot()
                .await?;

            Ok(out)
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
use playwright::api::ProxySettings;

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