#!/usr/bin/env python3
"""Capture the Microsoft Store screenshots from the real application.

Every scene opens a temporary copy of a working example design (the WebServer
designs compile and serve HTTP in `tests/c17_workflows.rs`) in its own
`wirecraft-desktop` process on a private Xvfb display with Mesa's software Vulkan
(lavapipe). Input goes only to that private display, never to the desktop session.
Scenes preselect their view through `ui.json`; popups and refreshes use real
clicks and keys, and the MCP scene makes real MCP requests to the instance's own
backend (its bearer token is read from that owned process and never printed or
stored). The scene projects are saved under packaging/microsoft-store/captures/.

Requires Xvfb, xdotool, ImageMagick, Mesa lavapipe, clangd, and a built
`target/debug/wirecraft-desktop` (or WIRECRAFT_STORE_BINARY).
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
STORE = ROOT / "packaging/microsoft-store"
SHOTS = STORE / "assets/screenshots"
CAPTURES = STORE / "captures"
BINARY = Path(os.environ.get("WIRECRAFT_STORE_BINARY", ROOT / "target/debug/wirecraft-desktop"))
DISPLAY = ":87"
WIDTH, HEIGHT = 1920, 1080
LAVAPIPE = "/usr/share/vulkan/icd.d/lvp_icd.json"


def example(name):
    """The committed example, never the working-tree copy with local edits."""
    return subprocess.check_output(["git", "show", "HEAD:examples/" + name], cwd=ROOT)


def named(document, name):
    document = json.loads(document) if isinstance(document, bytes) else document
    return next(n["id"] for n in document["nodes"] if n["name"] == name)


def kanban(document):
    """Ownership a team would record on the board; statuses stay as designed."""
    owners = {
        "lib_http_server": ("Platform team", "http-runtime"),
        "base_lib": ("Platform team", "http-runtime"),
        "socket_accept": ("Platform team", "networking"),
        "socket_respond": ("Platform team", "networking"),
        "router": ("Web team", "routing"),
        "routes": ("Web team", "routing"),
        "routeHomeAbout": ("Web team", "pages"),
        "routeHomeVersion": ("Web team", "pages"),
        "routeApiUserDetails": ("API team", "users-api"),
        "routeNotFound": ("Web team", "pages"),
        "ReqRep": ("API team", "contracts"),
        "Request": ("API team", "contracts"),
        "Response": ("API team", "contracts"),
    }
    for node in document["nodes"]:
        if node["name"] in owners:
            node["responsible"], node["assignee"] = owners[node["name"]]
    return document


# Each scene: files to place (name -> bytes or transform), ui.json state, and steps.
def scenes():
    # Unchanged examples are written byte for byte: a Reference pins its component's SHA-256.
    rust = example("webserver-rust.orch")
    component = example("webserver-component.orch")
    consumer = example("webserver.orch")
    fit = [("click", 745, 148), ("wait", 3)]
    return {
        "01-designer-topology": dict(
            files={"webserver-rust.orch": rust}, open="webserver-rust.orch",
            ui=dict(tab="Designer", selected_ids=[named(rust, "router")]), steps=fit),
        # The wheel zooms about the pointer: router, the routes composite and typed links.
        "02-designer-routing": dict(
            files={"webserver-rust.orch": rust}, open="webserver-rust.orch",
            ui=dict(tab="Designer", selected_ids=[named(rust, "routeApiUserDetails")]),
            steps=fit + [("wheel", 640, 330, 2), ("wait", 3)]),
        "03-component-ports": dict(
            files={"webserver-component.orch": component}, open="webserver-component.orch",
            ui=dict(tab="Designer"), steps=fit),
        "04-component-reference": dict(
            files={"webserver-component.orch": component, "webserver.orch": consumer},
            open="webserver.orch", ui=dict(tab="Designer"), steps=fit),
        "05-editor-generated-header": dict(
            files={"webserver-component.orch": component}, open="webserver-component.orch",
            ui=dict(tab="Editor", section="source", selected_ids=[named(component, "socket_respond")]),
            steps=[("wait", 6)]),
        "06-editor-completion": dict(
            files={"webserver-component.orch": component}, open="webserver-component.orch",
            ui=dict(tab="Editor", section="source", selected_ids=[named(component, "socket_respond")]),
            # A new statement after line 6, completing the lent library's members.
            steps=[("wait", 6), ("click", 700, 500), ("key", "End"), ("key", "Return"),
                   ("type", "item.response.body = libhttp->"), ("wait", 1),
                   ("key", "ctrl+space"), ("wait", 8)]),
        "07-editor-hover": dict(
            files={"webserver-rust.orch": rust}, open="webserver-rust.orch",
            ui=dict(tab="Editor", section="source", selected_ids=[named(rust, "socket_respond")]),
            steps=[("wait", 25), ("hover", "HOVER")]),
        "08-export-plan": dict(
            files={"webserver-rust.orch": rust}, open="webserver-rust.orch",
            ui=dict(tab="Export"), steps=[("click", 1842, 152), ("wait", 30)]),
        "09-project-management": dict(
            files={"webserver-rust.orch": (json.dumps(kanban(json.loads(rust)), indent=2) + "\n").encode()},
            open="webserver-rust.orch", ui=dict(tab="Project Management"), steps=[("wait", 2)]),
        "10-mcp-server": dict(
            files={"webserver-rust.orch": rust}, open="webserver-rust.orch",
            ui=dict(tab="MCP"), steps=[("mcp",), ("wait", 2), ("click", 344, 266), ("wait", 3)]),
        # Extra captures for the promotional artwork.
        "extra-analysis": dict(
            files={"webserver-rust.orch": rust}, open="webserver-rust.orch",
            ui=dict(tab="Analysis"), steps=[("click", 1745, 153), ("wait", 5)]),
        "extra-archetypes": dict(
            files={"webserver-rust.orch": rust}, open="webserver-rust.orch",
            ui=dict(tab="Archetypes"), steps=[("click", 50, 152), ("wait", 3), ("click", 90, 250), ("wait", 4)]),
    }


def mcp_session(app_pid, env):
    """A short agent session against the instance's own backend, filling its access log."""
    backend = subprocess.check_output(["pgrep", "-P", str(app_pid)], text=True).split()[0]
    argv = Path("/proc", backend, "cmdline").read_bytes().split(b"\0")
    token = argv[argv.index(b"--token") + 1].decode()
    # Only this backend's own listening socket: match its socket inodes, so no other
    # loopback service (such as a running Wirecraft or Threadworks) is contacted.
    inodes = set()
    for fd in Path("/proc", backend, "fd").iterdir():
        target = os.readlink(fd)
        if target.startswith("socket:["):
            inodes.add(target[8:-1])
    port = None
    for line in Path("/proc", backend, "net/tcp").read_text().splitlines()[1:]:
        fields = line.split()
        if fields[3] == "0A" and fields[9] in inodes and fields[1].startswith("0100007F:"):
            port = int(fields[1].split(":")[1], 16)
            break
    if port is None:
        raise RuntimeError("The owned backend has no loopback listener")
    calls = [
        ("initialize", {"protocolVersion": "2025-06-18", "capabilities": {},
                        "clientInfo": {"name": "store-capture-agent", "version": "1.0"}}),
        ("tools/list", {}),
        ("tools/call", {"name": "threadwork.get_design", "arguments": {}}),
        ("tools/call", {"name": "threadwork.list_entities", "arguments": {}}),
        ("tools/call", {"name": "threadwork.validate_design", "arguments": {}}),
        ("tools/call", {"name": "wirecraft.analyze_network", "arguments": {}}),
        ("tools/call", {"name": "wirecraft.component_fiches", "arguments": {}}),
    ]
    results = []
    for number, (method, params) in enumerate(calls, 1):
        body = json.dumps({"jsonrpc": "2.0", "id": number, "method": method, "params": params}).encode()
        request = urllib.request.Request(
            f"http://127.0.0.1:{port}/mcp", data=body, method="POST",
            headers={"Authorization": "Bearer " + token, "Content-Type": "application/json",
                     "Accept": "application/json, text/event-stream"})
        with urllib.request.urlopen(request, timeout=60) as reply:
            results.append(dict(method=method, tool=params.get("name"), status=reply.status))
        time.sleep(0.4)
    return results


def window_of(pid, env):
    for _ in range(80):
        found = subprocess.run(["xdotool", "search", "--onlyvisible", "--pid", str(pid)],
                               env=env, capture_output=True, text=True)
        if found.stdout.strip():
            return found.stdout.split()[0]
        time.sleep(0.25)
    raise RuntimeError("No window")


def capture(name, scene, env, work, hover):
    state = work / name
    shutil.rmtree(state, ignore_errors=True)
    state.mkdir(parents=True)
    for file, data in scene["files"].items():
        (state / file).write_bytes(data)
    opened = json.loads((state / scene["open"]).read_text())
    ui = dict(document_id=opened["id"], viewport=dict(x=40, y=40, zoom=1))
    ui.update(scene["ui"])
    if "selected_ids" in ui:
        ui["selected"] = ui["selected_ids"][0]
    (state / "ui.json").write_text(json.dumps(ui))
    log = (state / "app.log").open("w")
    app = subprocess.Popen([str(BINARY), str(state / scene["open"]), "--state-dir", str(state)],
                           env=env, stdout=log, stderr=subprocess.STDOUT)
    receipt = dict(scene=name, project=scene["open"], ui=scene["ui"], steps=[])
    try:
        window = window_of(app.pid, env)
        subprocess.run(["xdotool", "windowsize", "--sync", window, str(WIDTH), str(HEIGHT)], env=env, check=True)
        subprocess.run(["xdotool", "windowmove", window, "0", "0"], env=env, check=True)
        # The private display has no other client; focusing this window takes nothing from a user.
        subprocess.run(["xdotool", "windowfocus", window], env=env, check=True)
        time.sleep(8)
        for step in scene["steps"]:
            kind = step[0]
            if kind == "wait":
                time.sleep(step[1])
            elif kind == "click":
                subprocess.run(["xdotool", "mousemove", str(step[1]), str(step[2]), "click", "1"], env=env, check=True)
                # Without a window manager nothing gives the clicked window the X keyboard focus.
                subprocess.run(["xdotool", "windowfocus", window], env=env, check=True)
                time.sleep(1.5)
            elif kind == "wheel":
                subprocess.run(["xdotool", "mousemove", str(step[1]), str(step[2])], env=env, check=True)
                for _ in range(step[3]):
                    subprocess.run(["xdotool", "click", "4"], env=env, check=True)
                    time.sleep(0.3)
            elif kind == "key":
                subprocess.run(["xdotool", "key", step[1]], env=env, check=True)
                time.sleep(0.5)
            elif kind == "type":
                subprocess.run(["xdotool", "type", "--delay", "40", step[1]], env=env, check=True)
            elif kind == "hover":
                x, y = hover
                subprocess.run(["xdotool", "mousemove", str(x - 20), str(y)], env=env, check=True)
                time.sleep(0.3)
                subprocess.run(["xdotool", "mousemove", str(x), str(y)], env=env, check=True)
                time.sleep(6)
            elif kind == "mcp":
                receipt["mcp"] = mcp_session(app.pid, env)
            receipt["steps"].append(list(step))
        path = SHOTS / (name + ".png") if not name.startswith("extra-") else STORE / "assets/extra" / (name + ".png")
        path.parent.mkdir(parents=True, exist_ok=True)
        raw = state / "raw.png"
        subprocess.run(["import", "-display", DISPLAY, "-window", "root", str(raw)], env=env, check=True)
        colors = int(subprocess.check_output(["identify", "-format", "%k", str(raw)], text=True))
        if colors < 100:
            raise RuntimeError(f"{name}: the window has not painted ({colors} colors)")
        # Strip timestamps so unchanged pixels give unchanged files.
        subprocess.run(["convert", str(raw), "-strip", "-define", "png:exclude-chunks=date,time", str(path)], check=True)
        receipt.update(file=path.relative_to(STORE).as_posix(), sha256=hashlib.sha256(path.read_bytes()).hexdigest())
        print(path, flush=True)
    finally:
        app.terminate()
        try:
            app.wait(10)
        except subprocess.TimeoutExpired:
            app.kill()
            app.wait()
        log.close()
    # Keep the exact projects the screenshots show.
    target = CAPTURES / name
    shutil.rmtree(target, ignore_errors=True)
    target.mkdir(parents=True)
    for file in scene["files"]:
        shutil.copyfile(state / file, target / file)
    return receipt


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scene", action="append", help="capture only these scenes")
    parser.add_argument("--hover", default="730,680", help="x,y of the hovered symbol in 07-editor-hover")
    args = parser.parse_args()
    hover = tuple(int(v) for v in args.hover.split(","))
    env = dict(os.environ, DISPLAY=DISPLAY, VK_DRIVER_FILES=LAVAPIPE, VK_ICD_FILENAMES=LAVAPIPE)
    env.pop("WAYLAND_DISPLAY", None)
    version = subprocess.check_output([str(BINARY), "--version"], text=True).strip()
    xvfb = subprocess.Popen(["Xvfb", DISPLAY, "-screen", "0", f"{WIDTH}x{HEIGHT}x24", "-ac", "-noreset"],
                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    receipts_path = CAPTURES / "capture-receipts.json"
    receipts = json.loads(receipts_path.read_text()) if receipts_path.exists() else {}
    try:
        time.sleep(1.5)
        selected = scenes()
        with tempfile.TemporaryDirectory(prefix="wirecraft-store-") as temp:
            for name, scene in selected.items():
                if args.scene and name not in args.scene:
                    continue
                try:
                    receipt = capture(name, scene, env, Path(temp), hover)
                except RuntimeError as error:
                    # Software Vulkan occasionally presents a first frame late.
                    print(error, "- retrying", file=sys.stderr)
                    receipt = capture(name, scene, env, Path(temp), hover)
                receipt.update(application=version, display="private Xvfb 1920x1080, Mesa lavapipe",
                               source="examples at git HEAD " + subprocess.check_output(
                                   ["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, text=True).strip())
                receipts[name] = receipt
    finally:
        xvfb.terminate()
        CAPTURES.mkdir(parents=True, exist_ok=True)
        receipts_path.write_text(json.dumps(receipts, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    sys.exit(main())
