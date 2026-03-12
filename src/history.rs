use crate::edit::EditList;

pub struct EditHistory {
    undo_stack: Vec<EditList>,
    redo_stack: Vec<EditList>,
}

impl EditHistory {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn push(&mut self, state: EditList) {
        self.undo_stack.push(state);
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: EditList) -> Option<EditList> {
        let prev = self.undo_stack.pop()?;
        self.redo_stack.push(current);
        Some(prev)
    }

    pub fn redo(&mut self, current: EditList) -> Option<EditList> {
        let next = self.redo_stack.pop()?;
        self.undo_stack.push(current);
        Some(next)
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
