#!/usr/bin/env python3
"""Regenerate the Wirecraft compiler-template overlay from the baseline templates.

The 90 Threadworks 2.6.7 templates under crates/wirecraft-core/resources/compiler-templates
stay byte-identical (see PROVENANCE.json). Every deliberate Wirecraft divergence is an
exact replacement below; the generated files under compiler-templates-wirecraft take
precedence when a built-in set is loaded. A replacement whose original text is missing
fails, so baseline drift cannot silently drop an adaptation.

Adaptation 1 — library instances: a service library declares one shared instance of its
exported functions; each dependency link lends that instance to its consumer as an
argument named after the link, instead of per-link prefixed aliases or per-link values.
"""
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
BASE = ROOT / "crates/wirecraft-core/resources/compiler-templates"
OVERLAY = ROOT / "crates/wirecraft-core/resources/compiler-templates-wirecraft"

LIBRARY_LINK_COMMENT = (
    "{# Wirecraft: the service library declares one shared instance; the link only names "
    "the argument that lends it to the consumer. #}\n"
)
SERVICE_LIBRARY = (
    "{# Wirecraft: library source at file scope, followed by its single shared instance. #}\n"
    "{{ nodeProvenanceComment }}\n{{ declaration }}\n\n{{ libraryInstance }}\n"
)
EMPTY = "{# Intentionally empty: libraries are not initialized as processing entities. #}"

GO_PARAM = (
    "{% for link in dependencyInjectionLinks %}{% if not link.isLibraryCapability %}, {{ link.argumentSymbol }} "
    "{% if link.isSourceCapability %}*{{ link.capabilityTypeSymbol }}{% else %}any{% endif %}{% endif %}{% endfor %}",
    "{% for link in dependencyInjectionLinks %}, {{ link.argumentSymbol }} "
    "{% if link.isSourceCapability %}*{{ link.capabilityTypeSymbol }}{% elseif link.isLibraryCapability %}"
    "*{{ link.libraryTypeSymbol }}{% else %}any{% endif %}{% endfor %}",
)
GO_CALL = (
    "{% for link in child.dependencyInjectionLinks %}{% if not link.isLibraryCapability %}, "
    "{% if link.isSourceCapability %}&{% endif %}{{ link.dependencySymbol }}{% endif %}{% endfor %}",
    "{% for link in child.dependencyInjectionLinks %}, "
    "{% if link.isSourceCapability %}&{% endif %}{{ link.dependencySymbol }}{% endfor %}",
)
KOTLIN_TYPE = (
    "{% if link.isSourceCapability or link.isRunnableCapability %}{{ link.capabilityTypeSymbol }}{% else %}Any?{% endif %}",
    "{% if link.isSourceCapability or link.isRunnableCapability %}{{ link.capabilityTypeSymbol }}"
    "{% elseif link.isLibraryCapability %}{{ link.libraryTypeSymbol }}{% else %}Any?{% endif %}",
)
KOTLIN_CHILD = (
    '{% if child.node.kind.name != "Note" and child.node.kind.name != "Type" %}',
    '{% if child.node.kind.name != "Note" and child.node.kind.name != "Type" and child.node.stereotype != "ServiceLibrary" %}',
)
KOTLIN_LINK = (
    "{% if link.isLibraryCapability %}val {{ link.dependencySymbol }}: {% if link.sourceNodeTypeSymbol is not empty %}"
    "{{ link.sourceNodeTypeSymbol }}{% else %}Any?{% endif %} = {% if link.libraryInstantiation is not empty %}"
    "{{ link.libraryInstantiation }}{% else %}null{% endif %}\n",
    "{% if link.isLibraryCapability %}" + LIBRARY_LINK_COMMENT,
)
JS_LINK = (
    "{% if link.isLibraryCapability %}const {{ link.dependencySymbol }} = {% if link.libraryInstantiation is not empty %}"
    "{{ link.libraryInstantiation }}{% else %}undefined{% endif %};\n",
    "{% if link.isLibraryCapability %}" + LIBRARY_LINK_COMMENT,
)
PHP_FILTERS = [
    ("{% if not link.isLibraryCapability %}, mixed ${{ link.argumentSymbol }}{% endif %}", ", mixed ${{ link.argumentSymbol }}"),
    ("{% if not link.isLibraryCapability %}, ${{ link.argumentSymbol }}{% endif %}", ", ${{ link.argumentSymbol }}"),
    ("{% if not link.isLibraryCapability %}, ${{ link.dependencySymbol }}{% endif %}", ", ${{ link.dependencySymbol }}"),
    (
        "{% if not link.isLibraryCapability %}    global ${{ link.dependencySymbol }};\n{% endif %}",
        "    global ${{ link.dependencySymbol }};\n",
    ),
]

# (set, file) -> list of (old, new); every occurrence of old is replaced, at least one must exist.
EDITS = {
    ("go", "processor.peb"): [GO_PARAM],
    ("go", "generator.peb"): [GO_PARAM],
    ("go", "composite.peb"): [GO_PARAM, GO_CALL],
    ("kotlin", "processor.peb"): [KOTLIN_TYPE, KOTLIN_CHILD],
    ("kotlin", "processor-single.peb"): [KOTLIN_TYPE, KOTLIN_CHILD],
    ("kotlin", "generator.peb"): [KOTLIN_TYPE],
    ("kotlin", "generator-single.peb"): [KOTLIN_TYPE],
    ("kotlin", "link-declaration.peb"): [KOTLIN_LINK],
    ("nodejs", "link-declaration.peb"): [JS_LINK],
    ("quickjs", "link-declaration.peb"): [JS_LINK],
    ("php", "processor.peb"): PHP_FILTERS,
    ("php", "generator.peb"): PHP_FILTERS,
    ("php", "composite.peb"): PHP_FILTERS,
}
# Files appended to (go/php already have a service-library role) or created new.
APPEND = {
    ("go", "service-library-declaration.peb"): "\n{{ libraryInstance }}\n",
    ("php", "service-library-declaration.peb"): "\n{{ libraryInstance }}\n",
}
NEW = {
    (s, "service-library-declaration.peb"): SERVICE_LIBRARY for s in ["kotlin", "nodejs", "quickjs"]
} | {(s, "empty.peb"): EMPTY for s in ["kotlin", "nodejs", "quickjs"]}
PROPERTIES = {
    s: "# Wirecraft overlay: service libraries render at file scope with their shared instance.\n"
    "template.service-library.declaration=service-library-declaration.peb\n"
    "template.service-library.instantiation=empty.peb\n"
    for s in ["kotlin", "nodejs", "quickjs"]
}


def main() -> int:
    outputs = {}
    for (set_id, name), edits in EDITS.items():
        text = (BASE / set_id / name).read_text()
        for old, new in edits:
            # PHP filters overlap; only patterns present in a given file are required.
            if old not in text:
                if edits is PHP_FILTERS:
                    continue
                print(f"{set_id}/{name}: baseline text not found:\n{old}", file=sys.stderr)
                return 1
            text = text.replace(old, new)
        if edits is PHP_FILTERS and "isLibraryCapability %}" in text.replace("{% elseif link.isLibraryCapability %}", ""):
            if "{% if not link.isLibraryCapability %}" in text:
                print(f"{set_id}/{name}: unhandled library filter remains", file=sys.stderr)
                return 1
        outputs[(set_id, name)] = text
    for (set_id, name), suffix in APPEND.items():
        outputs[(set_id, name)] = (BASE / set_id / name).read_text().rstrip("\n") + "\n" + suffix
    outputs.update(NEW)
    for set_id, text in PROPERTIES.items():
        outputs[(set_id, "compiler.properties")] = text
    if OVERLAY.exists():
        for path in sorted(OVERLAY.rglob("*"), reverse=True):
            if path.is_file() and path.suffix in (".peb", ".properties"):
                path.unlink()
    for (set_id, name), text in sorted(outputs.items()):
        path = OVERLAY / set_id / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        print(f"wrote {path.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
