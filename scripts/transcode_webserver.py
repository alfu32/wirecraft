#!/usr/bin/env python3
"""Transcode examples/WebServer.orch (C17) into the template-compiler technologies.

Usage: transcode_webserver.py [LANGUAGE ...]   (default: every directory present)

Each examples/webserver-transcoded/<language>/ holds that language's bodies as
`<entity>.<section>.<extension>` files (section: source or initialization). The
design keeps the C17 topology: the same entities, IDs, links, composite, layout and
IO marks. Only the technology, the bodies and the type fields change: field types
become the language-neutral `string`/`number` (nested types stay references), so
every type template can map them. Writes examples/webserver-<language>.orch.
"""
import copy, json, pathlib, sys, uuid

ROOT = pathlib.Path(__file__).resolve().parent.parent
SOURCE = ROOT / "examples/WebServer.orch"
BODIES = ROOT / "examples/webserver-transcoded"
# language directory -> (languageId, technologyId, display name)
TECHNOLOGIES = {
    "go": ("go", "go", "Go"),
    "kotlin": ("kotlin", "kotlin-jvm", "Kotlin"),
    "nodejs": ("javascript", "nodejs", "Node.js"),
    "php": ("php", "php", "PHP"),
    # Engine-neutral scripts (formerly the QuickJS set); this design runs on Bun.
    "systemjs": ("javascript", "systemjs", "systemJS"),
}
FIELD_TYPES = {"char *": "string", "const char *": "string", "int": "number", "string": "string"}


def transcode(language: str) -> pathlib.Path:
    language_id, technology_id, display = TECHNOLOGIES[language]
    d = json.loads(SOURCE.read_text())
    nodes = {n["id"]: n for n in d["nodes"]}
    by_name = {n["name"]: n for n in d["nodes"] if not n.get("link")}
    bodies = BODIES / language
    used = set()
    for path in sorted(bodies.iterdir()):
        entity, section = path.name.split(".")[:2]
        if entity not in by_name or section not in ("source", "initialization"):
            sys.exit(f"{path}: unknown entity or section")
        by_name[entity]["text"][section] = path.read_text().rstrip("\n")
        used.add((entity, section))
    for n in d["nodes"]:
        n["diagnostics"] = []
        if n.get("link") or n["kind"] == "Type":
            if n["kind"] == "Type":
                for field in n["typeDefinition"]["fields"]:
                    if field.get("typeId") in FIELD_TYPES:
                        field["typeId"] = FIELD_TYPES[field["typeId"]]
            continue
        # Every C body must have a counterpart; leaving C text behind would not compile.
        for section in ("source", "initialization"):
            if n["text"].get(section, "").strip() and (n["name"], section) not in used:
                sys.exit(f"{language}: {n['name']} {section} has no transcoded body")
    root = nodes[d["rootNodeId"]]
    name = display.replace(".", "")
    root["name"] = f"WebServer{name[0].upper()}{name[1:]}"
    root["technology"]["languageId"] = language_id
    root["technology"]["technologyId"] = technology_id
    root["fileLayoutStrategyId"] = "single-file"
    root["text"]["specification"] = (
        f"Transcoded from examples/WebServer.orch (C17) into {display} by scripts/transcode_webserver.py "
        f"to test the {display} generator. Same topology, routes, limits and access log; only the bodies, "
        "the libraries and the type field types differ. The C17 specification follows.\n\n"
        + root["text"]["specification"]
    )
    d["id"] = str(uuid.uuid5(uuid.NAMESPACE_URL, f"wirecraft:webserver:{language}"))
    d["name"] = root["name"]
    d["users"] = []
    out = ROOT / f"examples/webserver-{language}.orch"
    out.write_text(json.dumps(d, indent=2, ensure_ascii=False) + "\n")
    return out


if __name__ == "__main__":
    languages = sys.argv[1:] or sorted(p.name for p in BODIES.iterdir() if p.is_dir())
    for language in languages:
        print(transcode(language).relative_to(ROOT))
