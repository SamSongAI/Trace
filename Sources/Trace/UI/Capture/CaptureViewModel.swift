import Foundation

final class CaptureViewModel: ObservableObject {
    @Published var text: String = ""
    @Published var selectedSection: NoteSection = .note
    @Published var selectedThread: ThreadConfig? = nil
    @Published var fileTitle: String = ""
    @Published var pinned: Bool = false
    @Published var toastMessage: String?

    func resetInput() {
        text = ""
        fileTitle = ""
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
