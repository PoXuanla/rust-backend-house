mod models;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use models::{ChatPayload, Message as ChatMessage, ProConfig};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
// 1. 定義共享狀態
struct AppState {
    // 廣播頻道：所有訊息都會經過這裡
    tx: broadcast::Sender<ChatMessage>,
}

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel(100);
    let app_store = Arc::new(AppState { tx });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_store);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("🚀 聊天室已啟動：ws://127.0.0.1:3000/ws");
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (sender, mut receiver) = socket.split();

    let sender = Arc::new(Mutex::new(sender));

    let mut rx: broadcast::Receiver<ChatMessage> = state.tx.subscribe();

    let _ = sender
        .lock()
        .await
        .send(Message::Text("歡迎連線".to_string()))
        .await;

    let sender_clone = Arc::clone(&sender);
    let mut send_task: tokio::task::JoinHandle<()> = tokio::spawn(async move {
        while let Ok(chat_msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&chat_msg) {
                if sender_clone
                    .lock()
                    .await
                    .send(Message::Text(json))
                    .await
                    .is_err()
                {
                    break;
                }
            }
        }
    });

    let tx = state.tx.clone();
    let sender_clone = Arc::clone(&sender);
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            println!("📥 收到消息: {}", text); // 调试日志
            match serde_json::from_str::<ChatMessage>(&text) {
                Ok(chat_msg) => {
                    println!("✅ 解析成功，广播消息"); // 调试日志
                    let _ = tx.send(chat_msg);
                }
                Err(e) => {
                    println!("❌ 解析失败: {}", e); // 调试日志
                    let error_msg =
                        ChatMessage::System("格式錯誤：請發送正確的 JSON 格式".to_string());
                    if let Ok(json) = serde_json::to_string(&error_msg) {
                        println!("📤 发送错误消息: {}", json); // 调试日志
                        match sender_clone.lock().await.send(Message::Text(json)).await {
                            Ok(_) => println!("✅ 错误消息发送成功"),
                            Err(e) => println!("❌ 错误消息发送失败: {}", e),
                        }
                    }
                }
            }
        }
    });

    // 如果其中一個任務結束（例如使用者關掉視窗），就停止另一個任務
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
