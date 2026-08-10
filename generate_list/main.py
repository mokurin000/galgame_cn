#!/usr/bin/env python3

import asyncio
import hashlib
from pathlib import Path

import typer
import zendriver as zd
from loguru import logger


app = typer.Typer()


def screenshot_name(url: str) -> str:
    """Generate a deterministic screenshot filename from URL."""
    return hashlib.md5(url.encode()).hexdigest() + ".webp"


async def worker(browser: zd.Browser, url: str, caption: str | None):
    """Open a webpage, wait for rendering, capture screenshot, and generate markdown."""

    page = await browser.get(url, new_tab=True)

    await page.send(
        zd.cdp.emulation.set_device_metrics_override(
            width=1280,
            height=720,
            mobile=False,
            device_scale_factor=1.0,
        ),
    )

    # Wait until the initial page DOM is ready.
    # Some pages may never reach the state, so ignore timeout errors.
    try:
        await page.wait_for_ready_state("interactive", timeout=10)
    except Exception as e:
        logger.warning(
            "Failed waiting for ready state for {}: {}",
            url,
            e,
        )

    # Give lazy-loaded images and dynamic content time to appear.
    await asyncio.sleep(10)

    title = page.title

    filename = Path("res") / screenshot_name(url)

    logger.info(
        "Saving screenshot for {} -> {}",
        url,
        filename,
    )

    try:
        await page.save_screenshot(
            filename=str(filename),
            format="webp",
        )
    except Exception:
        logger.exception(
            "Failed saving screenshot for {}",
            url,
        )
        return None

    # Generate HTML figure when a custom caption is provided.
    if caption:
        return f"""
<figure class="image">
  <a href="{url}">
    <img src="{filename}" alt="{title}"></img>
  </a>
  <figcaption>{caption}</figcaption>
</figure>

"""

    # Default markdown image format.
    return f"[![{title}]({filename})]({url})\n\n"


async def main_async(
    sites: Path,
    output: Path,
    proxy: str | None,
):
    """Process all URLs and append generated markdown to output file."""

    # Read existing output so already processed URLs can be skipped.
    current = output.read_text(
        encoding="utf-8",
        errors="ignore",
    )

    sites_text = sites.read_text(
        encoding="utf-8",
        errors="ignore",
    )

    Path("res").mkdir(exist_ok=True)

    logger.info(
        "Starting browser (proxy={})",
        proxy,
    )

    browser = await zd.start(
        headless=False,
        proxy=proxy,
    )

    tasks = []

    # Build asynchronous screenshot tasks.
    for line in sites_text.splitlines():
        if not line.strip():
            continue

        parts = line.split(maxsplit=1)

        url = parts[0]

        if url in current:
            logger.debug(
                "Skipping already processed URL: {}",
                url,
            )
            continue

        caption = parts[1] if len(parts) > 1 else None

        tasks.append(
            worker(
                browser,
                url,
                caption,
            )
        )

    logger.info(
        "Processing {} URLs",
        len(tasks),
    )

    results = await asyncio.gather(
        *tasks,
        return_exceptions=True,
    )

    # Append generated markdown snippets.
    with output.open(
        "a",
        encoding="utf-8",
    ) as f:
        for result in results:
            if isinstance(result, Exception):
                logger.error(
                    "Worker failed: {}",
                    result,
                )
                continue

            if result:
                f.write(result)

    await browser.stop()

    logger.info("Browser stopped")


@app.command()
def run(
    sites: Path = typer.Option(
        ...,
        "--sites",
        "-s",
        exists=True,
        file_okay=True,
        dir_okay=False,
        help="Input file containing URLs",
    ),
    output: Path = typer.Option(
        ...,
        "--output",
        "-o",
        exists=True,
        file_okay=True,
        dir_okay=False,
        help="Output markdown file",
    ),
    proxy: str | None = typer.Option(
        None,
        "--proxy",
        "-p",
        help="Proxy server, example http://127.0.0.1:8080",
    ),
):
    """Capture screenshots from URLs and generate markdown."""

    asyncio.run(
        main_async(
            sites,
            output,
            proxy,
        )
    )


if __name__ == "__main__":
    app()
