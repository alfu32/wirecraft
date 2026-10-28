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

Adaptation 2 — Node.js event loop: the runner is asynchronous and yields to the event loop
after every tick, so sockets, timers and signal handlers run between ticks; the baseline's
synchronous loop never let them run. A tick that moved packets continues on setImmediate;
consecutive idle ticks wait 1, 2, 4 ... up to 25 ms (the C17 example's poll timeout), and
`threadworkRunner.wake()` ends such a wait as soon as an event handler has work.

Adaptation 3 — nested composites in single files: a nested composite renders the same
single-file assembly as the root, so the Node.js and QuickJS runtime (and the QuickJS
banner) are emitted for the compilation root only; the baseline declared the runtime
classes once per composite, which is a redeclaration error.
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
NODE_RUN = [
    (
        "function threadworkRun() {\n  const context = createRuntimeContext();",
        "let threadworkIdleTicks = 0;\n\n"
        "/** Lets sockets, timers and signals run between ticks. Consecutive idle ticks back off to 25 ms;\n"
        " * threadworkRunner.wake() ends the wait as soon as an event handler has work. */\n"
        "function threadworkYield(moved) {\n"
        "  if (moved) {\n    threadworkIdleTicks = 0;\n    return new Promise((resolve) => setImmediate(resolve));\n  }\n"
        "  const delay = Math.min(25, 2 ** Math.min(threadworkIdleTicks++, 5));\n"
        "  return new Promise((resolve) => {\n"
        "    const timer = setTimeout(() => {\n      threadworkRunner.idleWake = null;\n      resolve();\n    }, delay);\n"
        "    threadworkRunner.idleWake = () => {\n      clearTimeout(timer);\n      resolve();\n    };\n"
        "  });\n"
        "}\n\n"
        "async function threadworkRun() {\n  const context = createRuntimeContext();",
    ),
    (
        "  while (threadworkRunner.isRunning()) {\n    {{ runSymbol }}(context);\n  }",
        "  while (threadworkRunner.isRunning()) {\n    const transit = threadworkRunner.transit;\n"
        "    {{ runSymbol }}(context);\n    await threadworkYield(threadworkRunner.transit !== transit);\n  }",
    ),
    (
        "  while (threadworkRunner.hasRecentTransit()) {\n    {{ runSymbol }}(context);\n  }",
        "  while (threadworkRunner.hasRecentTransit()) {\n    const transit = threadworkRunner.transit;\n"
        "    {{ runSymbol }}(context);\n    await threadworkYield(threadworkRunner.transit !== transit);\n  }",
    ),
    (
        "if (require.main === module) threadworkRun();",
        "if (require.main === module) {\n  threadworkRun().catch((error) => {\n    console.error(error);\n"
        "    process.exitCode = 1;\n  });\n}",
    ),
]
NODE_WAKE = (
    "  recordTransit() {\n    this.transit += 1;\n  }\n",
    "  recordTransit() {\n    this.transit += 1;\n  }\n\n"
    "  /** End an idle wait between ticks early; event handlers call it when they have work. */\n"
    "  wake() {\n    const wake = this.idleWake;\n    this.idleWake = null;\n    if (wake) wake();\n  }\n",
)
SINGLE_RUNTIME = (
    "{{ runtimeSupport }}\n",
    "{% if isCompilationRoot %}{{ runtimeSupport }}{% endif %}\n",
)
QUICKJS_BANNER = (
    "/* Generated by Threadwork's QuickJS compiler. */\n",
    "{% if isCompilationRoot %}/* Generated by Threadwork's QuickJS compiler. */{% endif %}\n",
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
    ("nodejs", "assembly-single.peb"): NODE_RUN + [SINGLE_RUNTIME],
    ("quickjs", "assembly-single.peb"): [SINGLE_RUNTIME, QUICKJS_BANNER],
    ("nodejs", "assembly-file.peb"): NODE_RUN,
    ("nodejs", "runtime.peb"): [NODE_WAKE],
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
