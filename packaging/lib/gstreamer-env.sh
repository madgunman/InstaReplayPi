# shellcheck shell=bash
# Source this before running replay-engine. Sets GStreamer plugin paths per OS.
# Usage: source "$(dirname "$0")/../packaging/lib/gstreamer-env.sh"

_gst_env() {
  case "$(uname -s)" in
    Darwin)
      if [[ -d /opt/homebrew/lib/gstreamer-1.0 ]]; then
        export GST_PLUGIN_PATH="/opt/homebrew/lib/gstreamer-1.0${GST_PLUGIN_PATH:+:$GST_PLUGIN_PATH}"
        export PKG_CONFIG_PATH="/opt/homebrew/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
        export DYLD_LIBRARY_PATH="/opt/homebrew/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
      elif [[ -d /usr/local/lib/gstreamer-1.0 ]]; then
        export GST_PLUGIN_PATH="/usr/local/lib/gstreamer-1.0${GST_PLUGIN_PATH:+:$GST_PLUGIN_PATH}"
        export PKG_CONFIG_PATH="/usr/local/lib/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
        export DYLD_LIBRARY_PATH="/usr/local/lib${DYLD_LIBRARY_PATH:+:$DYLD_LIBRARY_PATH}"
      fi
      ;;
    Linux)
      local bases=(
        /usr/lib/aarch64-linux-gnu/gstreamer-1.0
        /usr/lib/x86_64-linux-gnu/gstreamer-1.0
        /usr/lib/gstreamer-1.0
      )
      for b in "${bases[@]}"; do
        if [[ -d "$b" ]]; then
          export GST_PLUGIN_PATH="$b${GST_PLUGIN_PATH:+:$GST_PLUGIN_PATH}"
        fi
      done
      ;;
    MINGW*|MSYS*|CYGWIN*)
      if [[ -n "${GSTREAMER_ROOT:-}" && -d "$GSTREAMER_ROOT" ]]; then
        export PATH="$GSTREAMER_ROOT/bin:$PATH"
        export GST_PLUGIN_PATH="$GSTREAMER_ROOT/lib/gstreamer-1.0"
      elif [[ -d "C:/gstreamer/1.0/msvc_x86_64" ]]; then
        export PATH="C:/gstreamer/1.0/msvc_x86_64/bin:$PATH"
        export GST_PLUGIN_PATH="C:/gstreamer/1.0/msvc_x86_64/lib/gstreamer-1.0"
      fi
      ;;
  esac
}

_gst_env
