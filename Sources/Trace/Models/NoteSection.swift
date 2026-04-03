import Foundation

struct NoteSection: Identifiable, Hashable {
    static let minimumCount = 1
    static let maximumCount = 9
    static let defaultTitles = ["Note", "Memo", "Link", "Task"]
    static let legacyDefaultTitles = ["Note", "Clip", "Link", "Task", "Project"]

    static let note = NoteSection(index: 0, title: legacyDefaultTitles[0])
    static let clip = NoteSection(index: 1, title: legacyDefaultTitles[1])
    static let link = NoteSection(index: 2, title: legacyDefaultTitles[2])
    static let task = NoteSection(index: 3, title: legacyDefaultTitles[3])
    static let project = NoteSection(index: 4, title: legacyDefaultTitles[4])

    let index: Int
    let title: String

    var id: String { "\(index)-\(title)" }

    var displayIndex: Int {
        index + 1
    }

    var header: String {
        "# \(title)"
    }

    var shortcutLabel: String {
        "⌘\(displayIndex)"
    }

    static func == (lhs: NoteSection, rhs: NoteSection) -> Bool {
        lhs.index == rhs.index
    }

    func hash(into hasher: inout Hasher) {
        hasher.combine(index)
    }

    static func defaultTitle(for index: Int) -> String {
        "Section \(index + 1)"
    }
}
