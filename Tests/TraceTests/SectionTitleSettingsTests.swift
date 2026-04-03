import XCTest
@testable import Trace

final class SectionTitleSettingsTests: XCTestCase {
    func testSectionCanBeRenamed() {
        let defaults = makeDefaults(suffix: #function)
        let settings = AppSettings(defaults: defaults, fileManager: .default)

        settings.setTitle("Todo", for: .task)

        XCTAssertEqual(settings.title(for: .task), "Todo")
        XCTAssertEqual(defaults.stringArray(forKey: SettingKeys.sectionTitles)?[3], "Todo")
    }

    func testCurrentVersionPreservesStoredTodoProjectTitle() {
        let defaults = makeDefaults(suffix: #function)
        defaults.set(["Note", "Clip", "Link", "Task", "TODO"], forKey: SettingKeys.sectionTitles)
        defaults.set(2, forKey: SettingKeys.sectionTitlesOrderVersion)

        let settings = AppSettings(defaults: defaults, fileManager: .default)

        XCTAssertEqual(settings.title(for: .project), "TODO")
        XCTAssertEqual(defaults.stringArray(forKey: SettingKeys.sectionTitles), ["Note", "Clip", "Link", "Task", "TODO"])
    }

    func testLegacySectionTitleOrderMigrationPersistsUpdatedOrder() {
        let defaults = makeDefaults(suffix: #function)
        defaults.set(["Note", "Clip", "Link", "Fourth", "Fifth"], forKey: SettingKeys.sectionTitles)
        defaults.set(1, forKey: SettingKeys.sectionTitlesOrderVersion)

        let settings = AppSettings(defaults: defaults, fileManager: .default)

        XCTAssertEqual(settings.sectionTitles, ["Note", "Clip", "Link", "Fifth", "Fourth"])
        XCTAssertEqual(defaults.stringArray(forKey: SettingKeys.sectionTitles), ["Note", "Clip", "Link", "Fifth", "Fourth"])
        XCTAssertEqual(defaults.integer(forKey: SettingKeys.sectionTitlesOrderVersion), 2)
    }

    func testAddingSectionsStopsAtNine() {
        let defaults = makeDefaults(suffix: #function)
        let settings = AppSettings(defaults: defaults, fileManager: .default)

        for _ in 0..<10 {
            settings.addSection()
        }

        XCTAssertEqual(settings.sections.count, 9)
        XCTAssertEqual(settings.sections.last?.title, "Section 9")
        XCTAssertEqual(defaults.stringArray(forKey: SettingKeys.sectionTitles)?.count, 9)
    }

    func testRemovingSectionCompactsIndexesAndStopsAtOne() {
        let defaults = makeDefaults(suffix: #function)
        defaults.set(["One", "Two"], forKey: SettingKeys.sectionTitles)
        defaults.set(2, forKey: SettingKeys.sectionTitlesOrderVersion)

        let settings = AppSettings(defaults: defaults, fileManager: .default)

        settings.removeSection(settings.sections[0])
        settings.removeSection(settings.sections[0])

        XCTAssertEqual(settings.sections.count, 1)
        XCTAssertEqual(settings.sections[0].index, 0)
        XCTAssertEqual(settings.sections[0].title, "Two")
        XCTAssertEqual(defaults.stringArray(forKey: SettingKeys.sectionTitles), ["Two"])
    }

    private func makeDefaults(suffix: String) -> UserDefaults {
        let suiteName = "TraceTests.SectionTitleSettingsTests.\(suffix)"
        guard let defaults = UserDefaults(suiteName: suiteName) else {
            fatalError("Unable to create test defaults suite: \(suiteName)")
        }

        defaults.removePersistentDomain(forName: suiteName)
        return defaults
    }
}
