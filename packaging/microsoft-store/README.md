# Microsoft Store presentation

The English listing is in [english.md](english.md). Open [review.html](review.html)
in a browser to review it, then pick any other language to check its captions and
features. Store ID **9NDMQVXLGBQJ**; the product name (**Wirecraft**) and publisher
(**K3D0**) in [submission-fields.json](submission-fields.json) are assumptions to
confirm against Partner Center before importing.

This is a prepared import, **not a submitted Partner Center update**. The listing
describes Wirecraft **0.27.0 or later**. The application interface is English, and
the listing says so in every language. Translations have not had native-speaker
review.

## Build and import

From the repository root:

```sh
python3 scripts/microsoft_store.py --update-export   # build both imports, fill the export
python3 scripts/microsoft_store.py --check
python3 -m unittest discover -s scripts -p 'test_microsoft_store.py' -v
```

| Output under `packaging/microsoft-store/dist/` | Purpose |
| --- | --- |
| `wirecraft-store-english.zip` | English-only import (the export's neutral `en` column) |
| `wirecraft-store-all-languages.zip` | All 32 locale columns of the supplied export |
| `validation.json`, `SHA256SUMS.txt` | Validation record and ZIP checksums |

Unzip the chosen archive and select its **wirecraft-store** folder in the MSIX
submission's **Import listings** action. It contains one `listing.csv` and the
`images/` tree; the CSV's image paths include the root folder name, for example
`wirecraft-store/images/screenshots/01-designer-topology.png`. See
[Microsoft's import documentation](https://learn.microsoft.com/en-us/windows/apps/publish/publish-your-app/msix/import-and-export-store-listings).

The supplied export,
[listingData-9NDMQVXLGBQJ-1152921505702281824.csv](../listingData-9NDMQVXLGBQJ-1152921505702281824.csv),
is filled in place by `--update-export`; [template.csv](template.csv) keeps its
original Field, ID, Type, locale headers and row order. Edit the locale JSON files
under [locales/](locales) and rebuild, rather than editing a generated CSV.

## Listing coverage

Each of the 32 locales (`af ar bs-latn-ba bg zh cs da nl en et fil fi fr de el hi
it ja kk ko lt no pl pt ro ru-ua sr-latn es sv tr uk vi`) has a description, short
description, ten features, seven search terms, ten screenshot captions, and
minimum and recommended hardware. The title, publisher, copyright, license terms
and artwork are shared; the `default` column holds the English values. `zh` is
Simplified Chinese, `no` Norwegian Bokmål and `ru-ua` Russian for Ukraine. The
builder checks Microsoft's field limits (UTF-16 lengths, 21 search-term words) and
image sizes. Release notes, trailers and non-desktop images are intentionally
empty for this first listing.

The features lead with the software design capabilities (the dataflow designer,
routing, code generation in six languages, the body editor with its pinned
generated header, language-server help, navigation, components), followed by the
auxiliary ones (archetypes and analysis, project management, plan export for
printing, the local MCP server).

## Screenshots

**Ten unaltered 1920 × 1080 screenshots** of the real application, Wirecraft
0.27.0 (release build), in the listing's order:

| File | Shows |
| --- | --- |
| `01-designer-topology` | The Rust WebServer design: router, `routes` composite, service libraries, types |
| `02-designer-routing` | Zoomed routing into the composite, with typed link labels |
| `03-component-ports` | The C17 WebServer component with its Exterior ports |
| `04-component-reference` | The design that references that component, pinned by SHA-256 |
| `05-editor-generated-header` | A C17 body under its pinned generated function header, with the inspector |
| `06-editor-completion` | clangd completion of the lent library's members while typing |
| `07-editor-hover` | rust-analyzer hover over a library call in a Rust body |
| `08-export-plan` | The A2 plan preview with its title block, ready for PDF/SVG/PNG/Mermaid |
| `09-project-management` | The Kanban board with statuses, responsibility and assignees |
| `10-mcp-server` | The local MCP server after an agent session, with its access log |

The models are the repository's WebServer examples, which compile and serve HTTP
requests in `crates/wirecraft-runtime/tests/c17_workflows.rs`; the only additions
are the Kanban board's team ownership (`responsible`/`assignee`) in scene 09. The
exact projects each screenshot shows are saved under [captures/](captures), with
hashes, steps and the MCP requests in
[capture-receipts.json](captures/capture-receipts.json). Partner Center accepts at
most ten desktop screenshots; two more captures (network analysis and the
archetype library) appear inside the promotional artwork.

Capture again with (Linux; Xvfb, xdotool, ImageMagick, Mesa lavapipe, clangd and
rust-analyzer):

```sh
cargo build --release --locked -p wirecraft-desktop
WIRECRAFT_STORE_BINARY=target/release/wirecraft-desktop python3 scripts/capture_store_features.py
python3 scripts/prepare_store_art.py
```

The capture script runs every scene on a private Xvfb display with software
Vulkan, so it never sends input to, or takes focus from, the desktop session. The
MCP scene talks only to the backend that the capture started.

These are Linux captures; the Windows build has not been run yet (see
[../README.md](../README.md)).

## Artwork

| CSV field | PNG |
| --- | --- |
| StoreLogo300x300, StoreLogoOverride150x150 / 71x71 | The Wirecraft icon, transparent |
| StoreLogo720x1080 | Poster: routing, Rust hover, plan export, network analysis |
| StoreLogo1080x1080 | Square: topology, generated header, MCP server |
| PromoImage1920x1080 | Text-free hero: routing, component ports, completion, Kanban |
| OptionalPromo1000x800 / 358x358 / 414x180 | Designer, completion, Kanban and export; name and icon |

`scripts/prepare_store_art.py` composes them from unretouched crops of the
screenshots (recorded in `assets/art/sources.json`); the only added text is the
product name, so one set serves every language.

## Fields outside the CSV

[submission-fields.json](submission-fields.json) holds the prepared category
(**Developer tools**), pricing, website, support contact, Windows requirements and
certification notes. Still to do in Partner Center:

- **Privacy policy.** Enter the URL of [docs/PRIVACY_POLICY.md](../../docs/PRIVACY_POLICY.md)
  (https://github.com/alfu32/wirecraft/blob/master/docs/PRIVACY_POLICY.md).
  The listing answers Yes to personal information, since project files can carry
  people's names and MCP clients can read them; the policy says so.
- **Package.** Upload the release workflow's `wirecraft-X.Y.Z-windows-x64.msix`
  and `-arm64.msix` (0.27.4 or later; 0.27.3 produced no MSIX). They carry the identity in
  `packaging/windows/msix-identity.json`, copied from Partner Center's
  **Product identity**: `K3D0.wirecraft`, publisher `CN=7F0CBC95-…` and publisher
  display name `K3D0` (family name `K3D0.wirecraft_tfmtd4yrk952g`).
- Age rating questionnaire, markets, publisher declarations and the final
  submission are account-only steps; nothing here performs them.
