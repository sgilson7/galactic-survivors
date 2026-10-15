"""Two browser pages meet by invitation and use the co-op lobby (T24).

Run by hand after building, not from scripts/check.sh: it opens a WebRTC
connection between two pages, which the everyday check keeps out.

    .venv/bin/python tests/browser/lobby.py dist <folder for screenshots>
"""
import functools
import http.server
import sys
import threading
from playwright.sync_api import sync_playwright

DIST = sys.argv[1]
SHOTS = sys.argv[2]

handler = functools.partial(http.server.SimpleHTTPRequestHandler, directory=DIST)
handler.log_message = lambda *a: None
server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
origin = f"http://127.0.0.1:{server.server_address[1]}"

problems = []


def check(ok, what):
    print(("  PASS " if ok else "  FAIL ") + what)
    if not ok:
        problems.append(what)


def click(page, action, value=None):
    sel = f'button[data-action="{action}"]'
    if value is not None:
        sel += f'[data-value="{value}"]'
    page.locator(sel).first.click()


def ask_button(page, label):
    page.locator("#ask button", has_text=label).click()


with sync_playwright() as p:
    browser = p.chromium.launch()
    errors = []
    a_ctx = browser.new_context(viewport={"width": 1280, "height": 860})
    a_ctx.grant_permissions(["clipboard-read", "clipboard-write"], origin=origin)
    b_ctx = browser.new_context(viewport={"width": 390, "height": 844})
    a, b = a_ctx.new_page(), b_ctx.new_page()
    for pg in (a, b):
        pg.on("pageerror", lambda e: errors.append(str(e)))
        pg.on("console", lambda m: errors.append(m.text) if m.type == "error" else None)
        pg.goto(origin + "/")
        pg.wait_for_selector("body[data-ready]")
        click(pg, "screen.show", "coop")
    a.screenshot(path=f"{SHOTS}/lobby-door.png", full_page=True)
    check(a.locator('button[data-action="page.invite"]').count() == 0, "the invitation waits behind 'Can't connect with a code?'")
    click(a, "coop.more")
    click(a, "page.invite")
    a.wait_for_function("() => document.querySelector('#ask-text') && document.querySelector('#ask-text').value.length > 20", timeout=20000)
    invitation = a.locator("#ask-text").input_value()
    click(b, "coop.more")
    click(b, "page.reply")
    b.locator("#ask-text").fill(invitation)
    ask_button(b, "Answer an invitation")
    b.wait_for_function("(inv) => document.querySelector('#ask-text').value.length > 20 && document.querySelector('#ask-text').value !== inv", arg=invitation, timeout=20000)
    reply = b.locator("#ask-text").input_value()
    a.locator("#ask-text").fill(reply)
    ask_button(a, "Connect")
    a.wait_for_function("() => document.body.dataset.net === 'online.lobby'", timeout=20000)
    b.wait_for_function("() => document.body.dataset.net === 'online.lobby'", timeout=20000)
    check(True, "the two pages met by invitation and are in the lobby")
    b.locator("#ask").evaluate("e => e.hidden = true")
    a.locator("#ask").evaluate("e => e.hidden = true")
    check(a.locator('button[data-action="coop.start"]').is_disabled(), "the host's Start waits for Ready")
    check(a.locator('button[data-action="page.copy_code"]').count() == 0, "a room made by invitation offers no code to copy")

    # the partner names themselves, opens Gear, then presses Ready
    click(b, "page.name")
    b.locator("#ask-text").fill("  Peter  ")
    ask_button(b, "Change name")
    click(b, "coop.tab", "gear")
    b.wait_for_selector("#panel-inventory")
    click(b, "coop.ready")
    a.wait_for_selector('button[data-action="coop.start"]:not([disabled])', timeout=5000)
    partner = a.locator("#panel-room #item-partner").inner_text()
    check("peter" in partner.lower() and "changing gear" in partner.lower() and "ready" in partner.lower(), f"the host sees the partner's row: {partner!r}")
    a.screenshot(path=f"{SHOTS}/lobby-host.png", full_page=True)
    b.screenshot(path=f"{SHOTS}/lobby-partner-phone.png", full_page=True)
    widths = b.evaluate("() => [document.documentElement.scrollWidth, window.innerWidth]")
    check(widths[0] <= widths[1], f"no sideways scroll on a phone {widths}")

    # the host starts: both pages run
    click(a, "coop.start")
    a.wait_for_selector("body[data-screen=run]", timeout=5000)
    b.wait_for_selector("body[data-screen=run]", timeout=5000)
    check(True, "Start took both pages into the run")
    check(not errors, f"no page errors {errors}")

    # a room link lands in the co-op room and leaves the address clean
    c = a_ctx.new_page()
    c.goto(origin + "/#room=abcde")
    c.wait_for_selector("#panel-room", timeout=10000)
    check(c.evaluate("() => location.hash") == "", "the room is taken out of the address")
    check("ABCDE" in c.locator("#panel-room h2").inner_text(), "the link joined room ABCDE")
    click(c, "page.copy_code")
    c.wait_for_timeout(300)
    check(c.evaluate("() => navigator.clipboard.readText()") == "ABCDE", "Copy code copies the code")
    click(c, "page.copy_link")
    c.wait_for_timeout(300)
    link = c.evaluate("() => navigator.clipboard.readText()")
    check(link == origin + "/#room=ABCDE", f"Copy link copies a joining link {link!r}")
    c.screenshot(path=f"{SHOTS}/lobby-link.png", full_page=True)
    browser.close()

server.shutdown()
print("PROBLEMS:", problems if problems else "none")
sys.exit(1 if problems else 0)
