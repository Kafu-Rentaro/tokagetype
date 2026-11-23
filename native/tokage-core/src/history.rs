//! 文書スナップショットによる Undo/Redo 履歴。Web 版 `src/model/history.ts` に対応する。

use crate::model::ProjectDoc;

pub const HISTORY_LIMIT: usize = 100;

#[derive(Debug, Clone)]
pub struct History {
    past: Vec<ProjectDoc>,
    present: ProjectDoc,
    future: Vec<ProjectDoc>,
}

impl History {
    pub fn new(doc: ProjectDoc) -> Self {
        History {
            past: Vec::new(),
            present: doc,
            future: Vec::new(),
        }
    }

    pub fn present(&self) -> &ProjectDoc {
        &self.present
    }

    /// 履歴に記録せず現在文書を直接書き換えるための参照。
    /// 連続編集（ドラッグ等）では、開始前に [`History::record`] 用のスナップショットを
    /// 呼び出し側が保持し、終了時に record を呼ぶこと。
    pub fn present_mut(&mut self) -> &mut ProjectDoc {
        &mut self.present
    }

    /// 履歴を空にして別の文書に置き換える（ファイル読み込み時）。
    pub fn reset(&mut self, doc: ProjectDoc) {
        self.past.clear();
        self.future.clear();
        self.present = doc;
    }

    /// 離散的な変更を1ステップとして記録する（present を next に差し替える）。
    pub fn push(&mut self, next: ProjectDoc) {
        if next == self.present {
            return;
        }
        let old = std::mem::replace(&mut self.present, next);
        self.past.push(old);
        if self.past.len() > HISTORY_LIMIT {
            let excess = self.past.len() - HISTORY_LIMIT;
            self.past.drain(0..excess);
        }
        self.future.clear();
    }

    /// present が既に base から書き換えられている場合に、base を過去として記録する。
    /// ドラッグや連続入力の開始時スナップショットを終了時に確定するために使う。
    /// base と present が同一なら何もしない。
    pub fn record(&mut self, base: ProjectDoc) {
        if base == self.present {
            return;
        }
        self.past.push(base);
        if self.past.len() > HISTORY_LIMIT {
            let excess = self.past.len() - HISTORY_LIMIT;
            self.past.drain(0..excess);
        }
        self.future.clear();
    }

    pub fn undo(&mut self) -> bool {
        let Some(prev) = self.past.pop() else {
            return false;
        };
        let cur = std::mem::replace(&mut self.present, prev);
        self.future.insert(0, cur);
        true
    }

    pub fn redo(&mut self) -> bool {
        if self.future.is_empty() {
            return false;
        }
        let next = self.future.remove(0);
        let cur = std::mem::replace(&mut self.present, next);
        self.past.push(cur);
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }
}
