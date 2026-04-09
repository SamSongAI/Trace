// Deprecated: kept for backward compatibility. Use release-data.js + site.js.
window.release = window.TRACE_SITE ? {
  version: window.TRACE_SITE.current.version,
  releasedAt: window.TRACE_SITE.current.releasedAt,
  macOS: window.TRACE_SITE.current.platforms.macos,
  windows: window.TRACE_SITE.current.platforms.windows
} : null;
