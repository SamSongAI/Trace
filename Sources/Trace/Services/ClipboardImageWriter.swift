import AppKit
import Foundation

protocol ClipboardImageWritingSettingsProviding {
    var vaultPath: String { get }
    var dailyFolderName: String { get }
    var hasValidVaultPath: Bool { get }
}

enum ClipboardImageWriterError: LocalizedError, Equatable {
    case invalidVaultPath
    case imageEncodingFailed

    var errorDescription: String? {
        switch self {
        case .invalidVaultPath: return L10n.imageVaultNotConfigured
        case .imageEncodingFailed: return L10n.imageEncodingFailed
        }
    }
}

final class ClipboardImageWriter {
    private let settings: ClipboardImageWritingSettingsProviding
    private let fileManager: FileManager

    init(settings: ClipboardImageWritingSettingsProviding, fileManager: FileManager = .default) {
        self.settings = settings
        self.fileManager = fileManager
    }

    func saveFromPasteboardImage(_ image: NSImage, now: Date = Date()) throws -> String {
        guard settings.hasValidVaultPath else {
            throw ClipboardImageWriterError.invalidVaultPath
        }

        let baseDirectory = try assetsDirectory(for: now)
        try fileManager.createDirectory(at: baseDirectory, withIntermediateDirectories: true)

        let fileName = "trace-\(filenameTimestamp(for: now)).png"
        let targetURL = baseDirectory.appendingPathComponent(fileName, isDirectory: false)
        let data = try pngData(from: image)
        try data.write(to: targetURL, options: .atomic)

        let dateFolder = dayFolderString(for: now)
        let relativePath = "assets/\(dateFolder)/\(fileName)"
        return "![image](\(relativePath))"
    }

    private func assetsDirectory(for date: Date) throws -> URL {
        let trimmedVaultPath = settings.vaultPath.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedVaultPath.isEmpty else {
            throw ClipboardImageWriterError.invalidVaultPath
        }

        let vaultURL = URL(fileURLWithPath: trimmedVaultPath, isDirectory: true)
        return vaultURL
            .appendingPathComponent(settings.dailyFolderName, isDirectory: true)
            .appendingPathComponent("assets", isDirectory: true)
            .appendingPathComponent(dayFolderString(for: date), isDirectory: true)
    }

    private func pngData(from image: NSImage) throws -> Data {
        guard let tiff = image.tiffRepresentation,
              let bitmap = NSBitmapImageRep(data: tiff),
              let data = bitmap.representation(using: .png, properties: [:]) else {
            throw ClipboardImageWriterError.imageEncodingFailed
        }
        return data
    }

    private func dayFolderString(for date: Date) -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.string(from: date)
    }

    private func filenameTimestamp(for date: Date) -> String {
        let formatter = DateFormatter()
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyyMMdd-HHmmss-SSS"
        return formatter.string(from: date)
    }
}

extension AppSettings: ClipboardImageWritingSettingsProviding {}
