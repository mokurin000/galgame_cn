#!/usr/bin/env python3

import asyncio
import hashlib
from pathlib import Path

import typer
import zendriver as zd


app = typer.Typer()


CF_CLEARANCE = "CqCW39e7Dqzq4e74ac4NBDP_TkjJuvM6TtGCyEHqtQE-1723046672-1.0.1.1-_1Dj47lZPpDpiW6Iw6zb_lg3ZrmKgkJpxrRcxwhKWsXRtmHFy.YSBcCOYupK.I.ZSZ7tmJbfU729PlTb6K0NpQ"


def screenshot_name(url: str) -> str:
    return hashlib.md5(url.encode()).hexdigest() + ".webp"


async def worker(browser: zd.Browser, url: str, caption: str | None):
    page = await browser.get(url)

    try:
        await page.wait_for_ready_state("interactive", timeout=10)
    except Exception:
        pass

    if "ryuugames" in url:
        try:
            await page.verify_cf(timeout=10.0)
        except TimeoutError:
            pass
    # wait for lazy loading
    await asyncio.sleep(10)

    title = page.title

    filename = Path("res") / screenshot_name(url)

    print(f"screenshot path for {url}: {filename}")

    await page.save_screenshot(
        filename=str(filename),
        format="webp",
    )

    if caption:
        return f"""
<figure class="image">
  <a href="{url}">
    <img src="{filename}" alt="{title}"></img>
  </a>
  <figcaption>{caption}</figcaption>
</figure>

"""

    return f"[![{title}]({filename})]({url})\n\n"


async def main_async(
    sites: Path,
    output: Path,
    proxy: str | None,
):
    current = output.read_text(
        encoding="utf-8",
        errors="ignore",
    )

    sites_text = sites.read_text(
        encoding="utf-8",
        errors="ignore",
    )

    done = {line.split()[0] for line in current.splitlines() if line.strip()}

    Path("res").mkdir(exist_ok=True)

    browser = await zd.start(
        headless=False,
        proxy=proxy,
    )

    tasks = []

    for line in sites_text.splitlines():
        if not line.strip():
            continue

        parts = line.split(maxsplit=1)

        url = parts[0]

        if url in done:
            continue

        caption = parts[1] if len(parts) > 1 else None

        tasks.append(
            worker(
                browser,
                url,
                caption,
            )
        )

    results = await asyncio.gather(
        *tasks,
        return_exceptions=True,
    )

    with output.open(
        "a",
        encoding="utf-8",
    ) as f:
        for result in results:
            if isinstance(result, Exception):
                print("ERROR:", result)
                continue

            if result:
                f.write(result)

    await browser.stop()


@app.command()
def run(
    sites: Path = typer.Option(
        ...,
        "--sites",
        "-s",
        exists=True,
        file_okay=True,
        dir_okay=False,
    ),
    output: Path = typer.Option(
        ...,
        "--output",
        "-o",
        exists=True,
        file_okay=True,
        dir_okay=False,
    ),
    proxy: str | None = typer.Option(
        None,
        "--proxy",
        "-p",
        help="proxy, example http://127.0.0.1:8080",
    ),
):
    asyncio.run(
        main_async(
            sites,
            output,
            proxy,
        )
    )


if __name__ == "__main__":
    app()
