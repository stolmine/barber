use crate::edit::EditList;

pub struct EditHistory {
    undo_stack: Vec<(String, EditList)>,
    redo_stack: Vec<(String, EditList)>,
}

impl EditHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn push(&mut self, label: impl Into<String>, state: EditList) {
        self.undo_stack.push((label.into(), state));
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: EditList) -> Option<EditList> {
        let (label, prev) = self.undo_stack.pop()?;
        self.redo_stack.push((label, current));
        Some(prev)
    }

    pub fn redo(&mut self, current: EditList) -> Option<EditList> {
        let (label, next) = self.redo_stack.pop()?;
        self.undo_stack.push((label, current));
        Some(next)
    }

    pub fn peek_undo(&self) -> Option<&EditList> {
        self.undo_stack.last().map(|(_, el)| el)
    }

    pub fn undo_label(&self) -> Option<&str> {
        self.undo_stack.last().map(|(l, _)| l.as_str())
    }

    pub fn redo_label(&self) -> Option<&str> {
        self.redo_stack.last().map(|(l, _)| l.as_str())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}
