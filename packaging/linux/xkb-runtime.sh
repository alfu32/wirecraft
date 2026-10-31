#!/bin/sh
# The X11 bridge uses private core-library structures. Never mix runtime pairs.
# Return success when the ordinary loader search would choose an incomplete or
# split pair. A subshell preserves the caller's IFS and globbing configuration.
wirecraft_needs_bundled_xkb() (
    wirecraft_core_dir=
    wirecraft_bridge_dir=
    IFS=:
    set -f
    for wirecraft_directory in $1; do
        [ -n "$wirecraft_directory" ] || continue
        if [ -z "$wirecraft_core_dir" ] && [ -f "$wirecraft_directory/libxkbcommon.so.0" ]; then
            wirecraft_core_dir=$wirecraft_directory
        fi
        if [ -z "$wirecraft_bridge_dir" ] && [ -f "$wirecraft_directory/libxkbcommon-x11.so.0" ]; then
            wirecraft_bridge_dir=$wirecraft_directory
        fi
    done
    [ -z "$wirecraft_core_dir" ] || [ "$wirecraft_core_dir" != "$wirecraft_bridge_dir" ]
)
