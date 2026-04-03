import AppKit
import Carbon
import Foundation

struct KeyboardShortcut: Equatable {
    static let supportedModifierMask: UInt32 = UInt32(cmdKey | shiftKey | optionKey | controlKey)

    let keyCode: UInt32
    let modifiers: UInt32

    init(keyCode: UInt32, modifiers: UInt32) {
        self.keyCode = keyCode
        self.modifiers = Self.sanitizedCarbonModifiers(modifiers)
    }

    var displayLabel: String {
        Self.modifierSymbols(for: modifiers) + Self.keyLabel(for: keyCode)
    }

    var hasModifier: Bool {
        modifiers != 0
    }

    var isReservedSectionSwitch: Bool {
        modifiers & UInt32(cmdKey) != 0 && Self.sectionSwitchKeyCodes.contains(keyCode)
    }

    func matches(_ event: NSEvent) -> Bool {
        keyCode == UInt32(event.keyCode)
            && modifiers == Self.sanitizedCarbonModifiers(event.modifierFlags)
    }

    static func from(event: NSEvent) -> KeyboardShortcut {
        KeyboardShortcut(
            keyCode: UInt32(event.keyCode),
            modifiers: sanitizedCarbonModifiers(event.modifierFlags)
        )
    }

    static func sanitizedCarbonModifiers(_ modifiers: UInt32) -> UInt32 {
        modifiers & supportedModifierMask
    }

    static func sanitizedCarbonModifiers(_ flags: NSEvent.ModifierFlags) -> UInt32 {
        var result: UInt32 = 0
        let clean = flags.intersection([.command, .shift, .option, .control])
        if clean.contains(.command) { result |= UInt32(cmdKey) }
        if clean.contains(.shift) { result |= UInt32(shiftKey) }
        if clean.contains(.option) { result |= UInt32(optionKey) }
        if clean.contains(.control) { result |= UInt32(controlKey) }
        return result
    }

    private static let sectionSwitchKeyCodes: Set<UInt32> = [
        UInt32(kVK_ANSI_1),
        UInt32(kVK_ANSI_2),
        UInt32(kVK_ANSI_3),
        UInt32(kVK_ANSI_4),
        UInt32(kVK_ANSI_5),
        UInt32(kVK_ANSI_6),
        UInt32(kVK_ANSI_7),
        UInt32(kVK_ANSI_8),
        UInt32(kVK_ANSI_9)
    ]

    private static func modifierSymbols(for modifiers: UInt32) -> String {
        var parts: [String] = []
        if modifiers & UInt32(controlKey) != 0 { parts.append("⌃") }
        if modifiers & UInt32(optionKey) != 0 { parts.append("⌥") }
        if modifiers & UInt32(shiftKey) != 0 { parts.append("⇧") }
        if modifiers & UInt32(cmdKey) != 0 { parts.append("⌘") }
        return parts.joined()
    }

    private static func keyLabel(for keyCode: UInt32) -> String {
        if let label = keyCodeLabels[keyCode] {
            return label
        }
        return "Key-\(keyCode)"
    }

    private static let keyCodeLabels: [UInt32: String] = [
        UInt32(kVK_ANSI_A): "A",
        UInt32(kVK_ANSI_B): "B",
        UInt32(kVK_ANSI_C): "C",
        UInt32(kVK_ANSI_D): "D",
        UInt32(kVK_ANSI_E): "E",
        UInt32(kVK_ANSI_F): "F",
        UInt32(kVK_ANSI_G): "G",
        UInt32(kVK_ANSI_H): "H",
        UInt32(kVK_ANSI_I): "I",
        UInt32(kVK_ANSI_J): "J",
        UInt32(kVK_ANSI_K): "K",
        UInt32(kVK_ANSI_L): "L",
        UInt32(kVK_ANSI_M): "M",
        UInt32(kVK_ANSI_N): "N",
        UInt32(kVK_ANSI_O): "O",
        UInt32(kVK_ANSI_P): "P",
        UInt32(kVK_ANSI_Q): "Q",
        UInt32(kVK_ANSI_R): "R",
        UInt32(kVK_ANSI_S): "S",
        UInt32(kVK_ANSI_T): "T",
        UInt32(kVK_ANSI_U): "U",
        UInt32(kVK_ANSI_V): "V",
        UInt32(kVK_ANSI_W): "W",
        UInt32(kVK_ANSI_X): "X",
        UInt32(kVK_ANSI_Y): "Y",
        UInt32(kVK_ANSI_Z): "Z",
        UInt32(kVK_ANSI_0): "0",
        UInt32(kVK_ANSI_1): "1",
        UInt32(kVK_ANSI_2): "2",
        UInt32(kVK_ANSI_3): "3",
        UInt32(kVK_ANSI_4): "4",
        UInt32(kVK_ANSI_5): "5",
        UInt32(kVK_ANSI_6): "6",
        UInt32(kVK_ANSI_7): "7",
        UInt32(kVK_ANSI_8): "8",
        UInt32(kVK_ANSI_9): "9",
        UInt32(kVK_ANSI_Minus): "-",
        UInt32(kVK_ANSI_Equal): "=",
        UInt32(kVK_ANSI_LeftBracket): "[",
        UInt32(kVK_ANSI_RightBracket): "]",
        UInt32(kVK_ANSI_Backslash): "\\",
        UInt32(kVK_ANSI_Semicolon): ";",
        UInt32(kVK_ANSI_Quote): "'",
        UInt32(kVK_ANSI_Comma): ",",
        UInt32(kVK_ANSI_Period): ".",
        UInt32(kVK_ANSI_Slash): "/",
        UInt32(kVK_ANSI_Grave): "`",
        UInt32(kVK_Return): "Enter",
        UInt32(kVK_ANSI_KeypadEnter): "Enter",
        UInt32(kVK_Tab): "Tab",
        UInt32(kVK_Space): "Space",
        UInt32(kVK_Delete): "Delete",
        UInt32(kVK_ForwardDelete): "ForwardDelete",
        UInt32(kVK_Escape): "Esc",
        UInt32(kVK_Home): "Home",
        UInt32(kVK_End): "End",
        UInt32(kVK_PageUp): "PageUp",
        UInt32(kVK_PageDown): "PageDown",
        UInt32(kVK_LeftArrow): "Left",
        UInt32(kVK_RightArrow): "Right",
        UInt32(kVK_UpArrow): "Up",
        UInt32(kVK_DownArrow): "Down",
        UInt32(kVK_F1): "F1",
        UInt32(kVK_F2): "F2",
        UInt32(kVK_F3): "F3",
        UInt32(kVK_F4): "F4",
        UInt32(kVK_F5): "F5",
        UInt32(kVK_F6): "F6",
        UInt32(kVK_F7): "F7",
        UInt32(kVK_F8): "F8",
        UInt32(kVK_F9): "F9",
        UInt32(kVK_F10): "F10",
        UInt32(kVK_F11): "F11",
        UInt32(kVK_F12): "F12"
    ]
}
