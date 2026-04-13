import Foundation

struct ThreadConfig: Identifiable, Codable, Equatable {
    let id: UUID
    var name: String
    var targetFile: String
    var icon: String?
    var order: Int

    init(
        id: UUID = UUID(),
        name: String,
        targetFile: String,
        icon: String? = nil,
        order: Int = 0
    ) {
        self.id = id
        self.name = name
        self.targetFile = targetFile
        self.icon = icon
        self.order = order
    }

    static func == (lhs: ThreadConfig, rhs: ThreadConfig) -> Bool {
        lhs.id == rhs.id &&
        lhs.name == rhs.name &&
        lhs.targetFile == rhs.targetFile &&
        lhs.icon == rhs.icon &&
        lhs.order == rhs.order
    }

    static let `default` = ThreadConfig(
        name: "想法",
        targetFile: "想法.md",
        icon: "lightbulb"
    )
}
