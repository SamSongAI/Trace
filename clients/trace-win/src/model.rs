#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteSection {
    Note,
    Clip,
    Link,
    Task,
    Project,
}

impl NoteSection {
    pub const ALL: [NoteSection; 5] = [
        NoteSection::Note,
        NoteSection::Clip,
        NoteSection::Link,
        NoteSection::Task,
        NoteSection::Project,
    ];

    pub fn index(self) -> usize {
        match self {
            NoteSection::Note => 0,
            NoteSection::Clip => 1,
            NoteSection::Link => 2,
            NoteSection::Task => 3,
            NoteSection::Project => 4,
        }
    }

    pub fn shortcut_label(self) -> &'static str {
        match self {
            NoteSection::Note => "Ctrl+1",
            NoteSection::Clip => "Ctrl+2",
            NoteSection::Link => "Ctrl+3",
            NoteSection::Task => "Ctrl+4",
            NoteSection::Project => "Ctrl+5",
        }
    }

    pub fn default_title(self) -> &'static str {
        match self {
            NoteSection::Note => "Note",
            NoteSection::Clip => "Clip",
            NoteSection::Link => "Link",
            NoteSection::Task => "Task",
            NoteSection::Project => "Project",
        }
    }
}
