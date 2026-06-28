import Foundation

final class CaptureViewModel: ObservableObject {
    @Published var text: String = ""
    @Published var selectedSection: NoteSection = .note
    @Published var selectedThread: ThreadConfig? = nil
    @Published var fileTitle: String = ""
    @Published var pinned: Bool = false
    @Published var toastMessage: String?
    @Published var pastedImagePaths: [String] = []
    @Published var pastedImageMarkdowns: [String] = []
    @Published var isSending: Bool = false

    func resetInput() {
        text = ""
        fileTitle = ""
        pastedImagePaths = []
        pastedImageMarkdowns = []
    }

    func beginSendAnimation(completion: @escaping () -> Void) {
        isSending = true
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.25) { [weak self] in
            self?.isSending = false
            completion()
        }
    }

    func showToast(_ message: String, duration: TimeInterval = 1.5) {
        toastMessage = message
        DispatchQueue.main.asyncAfter(deadline: .now() + duration) { [weak self] in
            if self?.toastMessage == message {
                self?.toastMessage = nil
            }
        }
    }
}
