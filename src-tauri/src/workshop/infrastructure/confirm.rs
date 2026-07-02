//! workshop インフラ: 破壊的ツールの human-in-the-loop 確認ゲート。
//! ツールが確認要求（ConfirmRequest）を進捗イベントとして流し、ユーザーの応答
//! （workshop_confirm コマンド）・取消・超時のいずれかまで待つ。承認だけが true。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;

use crate::agent::StreamProgress;

/// 未応答の確認要求 id → 応答チャネルの共有表。lib.rs で app.manage し、
/// workshop_confirm コマンドが resolve で回填する。
pub type PendingConfirms = Arc<Mutex<HashMap<u64, oneshot::Sender<bool>>>>;

/// 要求 id の採番。プロセス内で一意なら十分。
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// 応答待ちの既定タイムアウト。超過は拒否扱い（実行を永久に塞がない）。
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

fn lock(pending: &PendingConfirms) -> MutexGuard<'_, HashMap<u64, oneshot::Sender<bool>>> {
  pending.lock().unwrap_or_else(|e| e.into_inner())
}

/// 破壊的ツール（update/delete）が実行前にユーザー確認を取るためのゲート。
/// 1 回の workshop_chat 実行ごとに tx / cancel を束ねて作り、ツールへ渡す。
pub struct ConfirmGate {
  pub pending: PendingConfirms,
  pub tx: UnboundedSender<StreamProgress>,
  pub cancel: Arc<AtomicBool>,
}

impl ConfirmGate {
  /// 確認要求を発行してユーザーの応答を待つ。承認だけ true。
  /// 取消（workshop_cancel）・超時・イベント送信失敗は全て拒否として false。
  pub async fn request(&self, summary: &str) -> bool {
    self.request_with(summary, DEFAULT_TIMEOUT).await
  }

  async fn request_with(&self, summary: &str, timeout: Duration) -> bool {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let (sender, mut receiver) = oneshot::channel();
    lock(&self.pending).insert(id, sender);
    let event = StreamProgress::ConfirmRequest { id, summary: summary.to_string() };
    if self.tx.send(event).is_err() {
      lock(&self.pending).remove(&id);
      return false;
    }
    let deadline = std::time::Instant::now() + timeout;
    // ponytail: 50ms ポーリングで取消フラグ（AtomicBool）と超時を見る。通知化は必要になってから。
    loop {
      match tokio::time::timeout(Duration::from_millis(50), &mut receiver).await {
        Ok(answer) => return answer.unwrap_or(false),
        Err(_) => {
          if self.cancel.load(Ordering::Relaxed) || std::time::Instant::now() >= deadline {
            lock(&self.pending).remove(&id);
            return false;
          }
        }
      }
    }
  }
}

/// workshop_confirm コマンドからの回填。未知 id は黙って無視（取消・超時済み）。
pub fn resolve(pending: &PendingConfirms, id: u64, approved: bool) {
  if let Some(sender) = lock(pending).remove(&id) {
    let _ = sender.send(approved);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn make_gate() -> (ConfirmGate, tokio::sync::mpsc::UnboundedReceiver<StreamProgress>) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let gate = ConfirmGate {
      pending: PendingConfirms::default(),
      tx,
      cancel: Arc::new(AtomicBool::new(false)),
    };
    (gate, rx)
  }

  #[tokio::test]
  async fn request_blocks_until_approved() {
    let (gate, mut rx) = make_gate();
    let pending = gate.pending.clone();
    let task = tokio::spawn(async move { gate.request("delete entries/a.md").await });

    // 確認要求イベントが流れ、id が採番されている。
    let Some(StreamProgress::ConfirmRequest { id, summary }) = rx.recv().await else {
      panic!("expected ConfirmRequest event");
    };
    assert_eq!(summary, "delete entries/a.md");

    resolve(&pending, id, true);
    assert!(task.await.unwrap());
    assert!(lock(&pending).is_empty());
  }

  #[tokio::test]
  async fn request_returns_false_on_denial_and_ignores_unknown_id() {
    let (gate, mut rx) = make_gate();
    let pending = gate.pending.clone();
    resolve(&pending, 9999, true); // 未知 id は無視（パニックしない）
    let task = tokio::spawn(async move { gate.request("update entries/a.md").await });

    let Some(StreamProgress::ConfirmRequest { id, .. }) = rx.recv().await else {
      panic!("expected ConfirmRequest event");
    };
    resolve(&pending, id, false);
    assert!(!task.await.unwrap());
  }

  #[tokio::test]
  async fn request_denies_on_cancel() {
    let (gate, _rx) = make_gate();
    gate.cancel.store(true, Ordering::Relaxed);
    assert!(!gate.request_with("x", Duration::from_secs(10)).await);
    assert!(lock(&gate.pending).is_empty());
  }

  #[tokio::test]
  async fn request_denies_on_timeout() {
    let (gate, _rx) = make_gate();
    assert!(!gate.request_with("x", Duration::from_millis(10)).await);
    assert!(lock(&gate.pending).is_empty());
  }
}
