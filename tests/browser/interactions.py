"""The project's own browser interactions, run by check.py in every engine.

check.py has loaded the page, waited for it to report ready, and let it
idle. These drive the game the way a person would and assert what the page
shows. Expected text comes from data/strings.json and data/content.json,
read here, never from the game's own output.
"""

import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[2]
STRINGS = json.loads((ROOT / "data" / "strings.json").read_text())
CONTENT = json.loads((ROOT / "data" / "content.json").read_text())


def s(path):
    node = STRINGS
    for part in path.split("."):
        node = node[part]
    return node


def run(page, check, engine):
    check(page.text_content("#title") == s("screen.title.title"), "the title screen shows the game's name")
    check(page.get_attribute("body", "data-screen") == "title", "the core's title screen is shown")

    # every achievement is listed before any is earned (I20)
    page.click("button[data-action='screen.show'][data-value='achievements']")
    page.wait_for_selector("body[data-screen='achievements']", state="attached")
    listed = page.locator("#panel-achievements .item").count()
    check(listed == len(CONTENT["achievements"]), f"all {len(CONTENT['achievements'])} achievements are listed ({listed})")
    page.click("button[data-action='screen.show'][data-value='title']")

    # the outfitter sells no XP or gold bonus (the person: "xp and gold: not allowed")
    page.click("button[data-action='screen.show'][data-value='shop']")
    page.wait_for_selector("body[data-screen='shop']", state="attached")
    shop_text = page.inner_text("#panel-shop").lower()
    check("xp" not in shop_text and "more gold" not in shop_text, "the outfitter offers no XP or gold bonus")
    page.click("button[data-action='screen.show'][data-value='title']")

    # a solo run starts and draws on the canvas
    page.click("button[data-action='screen.show'][data-value='hangar']")
    page.wait_for_selector("body[data-screen='hangar']", state="attached")
    locked = page.locator("#item-vector[data-tone='locked']").count()
    check(locked == 1, "Vector starts locked")
    page.click("button[data-action='run.start']")
    page.wait_for_selector("body[data-screen='run']", state="attached")
    check(page.is_visible("#view"), "the run is drawn on the canvas")
    page.keyboard.down("KeyD")
    page.wait_for_selector("#overlay:not([hidden]) button[data-action='card.take']", timeout=60000)
    page.keyboard.up("KeyD")
    cards = page.locator("#overlay .item").count()
    check(cards == CONTENT["run"]["card_count"], f"a level-up offers {CONTENT['run']['card_count']} cards ({cards})")
    page.keyboard.press("Digit1")
    page.wait_for_selector("#overlay", state="hidden")
    check(True, "a card is taken with the 1 key")

    # pausing and ending the run keeps the rewards
    page.keyboard.press("Escape")
    page.wait_for_selector("button[data-action='run.quit']")
    page.click("#overlay button[data-action='run.quit']")
    page.wait_for_selector("body[data-screen='results']", state="attached")
    check(page.text_content("#title") == s("results.lost"), "ending the run shows the results")
    check(s("results.save_reminder") not in page.inner_text("#panels"), "the browser keeps the save, so the results ask for no download (T25)")

    # the save downloads as a file and loads back
    with page.expect_download() as d:
        page.click("#panel-nav button[data-action='page.download']")
    path = d.value.path()
    saved = json.loads(pathlib.Path(path).read_text())
    check(saved.get("format") == "galactic-survivors-save", "the downloaded save is a Galactic Survivors save")
    check(saved.get("stats", {}).get("runs") == 1, "the save counts the run just played")
    check(s("save.done") in page.inner_text("#message"), "the page confirms the download")
    page.click("#panel-nav button[data-action='screen.show'][data-value='title']")
    with page.expect_file_chooser() as fc:
        page.click("#panel-save button[data-action='page.load']")
    fc.value.set_files(path)
    page.wait_for_selector(f"#message:has-text('{s('save.loaded')}')")
    check(True, "the save loads back from the file")

    # the browser kept the save: after a reload the run is still counted (T25)
    page.reload()
    page.wait_for_selector("body[data-ready]", state="attached")
    check(s("save.explain") in page.inner_text("#panel-save"), "the save panel says the browser keeps the save")
    with page.expect_download() as d:
        page.click("#panel-save button[data-action='page.download']")
    kept = json.loads(pathlib.Path(d.value.path()).read_text())
    check(kept.get("stats", {}).get("runs") == 1, "the save survives a reload")
