import Foundation

/// Safe resource bundle lookup that works in .app bundles.
///
/// SwiftPM's auto-generated `Bundle.module` looks for the resource bundle at
/// `Bundle.main.bundleURL/<name>.bundle` (i.e. `Trace.app/<name>.bundle`).
/// This works for command-line tools but NOT for `.app` bundles, where
/// resources live under `Contents/Resources/`.  On any machine without the
/// local `.build/` directory the accessor crashes with `fatalError`.
///
/// This helper checks `Contents/Resources/` first, then falls back to
/// `Bundle.module` only when the preferred path already exists (i.e. during
/// development or `swift run`).
enum AppResourceBundle {
    static let bundle: Bundle? = {
        let bundleName = "Trace_Trace"

        // 1. Standard .app location: Contents/Resources/
        if let resourceURL = Bundle.main.resourceURL {
            let path = resourceURL.appendingPathComponent("\(bundleName).bundle").path
            if let bundle = Bundle(path: path) {
                return bundle
            }
        }

        // 2. Alongside the executable (swift run / command-line)
        if let executableURL = Bundle.main.executableURL {
            let path = executableURL
                .deletingLastPathComponent()
                .appendingPathComponent("\(bundleName).bundle").path
            if let bundle = Bundle(path: path) {
                return bundle
            }
        }

        // 3. Bundle.main root (SwiftPM's default expectation)
        let mainPath = Bundle.main.bundleURL
            .appendingPathComponent("\(bundleName).bundle").path
        if let bundle = Bundle(path: mainPath) {
            return bundle
        }

        return nil
    }()

    /// Convenience: look up a resource URL, returning nil if the bundle is
    /// unavailable rather than crashing.
    static func url(forResource name: String, withExtension ext: String) -> URL? {
        bundle?.url(forResource: name, withExtension: ext)
    }
}
