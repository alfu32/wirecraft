# Wirecraft 0.6.3 box-only layout

Seven relevant integration tests cover all six layout commands, mixed-parent selected links, extreme link bounds, untouched link data, box-only response IDs, undo and insufficient-box failures. The real authenticated HTTP fixture repeats all six commands and undo in an owned temporary backend. Run `python3 layout_http.py /absolute/path/to/wirecraft` to reproduce. Optimized installed backend/MCP/PDF/C17/clangd checks pass.

See verification.json for exact commands, results and binary hashes. Native UI suites and the full workspace suite were not rerun for this patch. Live instances and user projects/settings were preserved.
