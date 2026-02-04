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
use models::{Message as ChatMessage};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::mpsc;
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
    // 拿到 ws 發送器跟接收器
    let (mut ws_sender, mut ws_receiver) = socket.split();
    // 啟用 broadcast 發送器
    let mut broadcast_rx = state.tx.subscribe();
    // 建立多對一處理器
    let (tx_out, mut rx_out) = mpsc::unbounded_channel::<Message>();
    // mpsc 發送一筆消息
    let _ = tx_out.send(Message::Text("歡迎連線".to_string()));

    
    // mpsc 接收到資料，將資料透過 ws 發送器發出去
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx_out.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    //廣播收到的內容同步給 mpsc
    let tx_out_clone = tx_out.clone();
    let mut broadcast_task = tokio::spawn(async move {
        while let Ok(chat_msg) = broadcast_rx.recv().await {
            if let Ok(json) = serde_json::to_string(&chat_msg){
                if tx_out_clone.send(Message::Text(json)).is_err(){
                    break;
                }
            }
        }
    });

    let broadcast_tx = state.tx.clone();
    let mut recv_task = tokio::spawn(async move {
       while let Some(Ok(Message::Text(text))) = ws_receiver.next().await {
        println!("📥 收到消息: {}", text);
        match serde_json::from_str::<ChatMessage>(&text){
            Ok(chat_msg) => {
              println!("✅ 解析成功，广播消息");
              let _ = broadcast_tx.send(chat_msg);
            }
            Err(e) => {
                println!("❌ 解析失败: {}", e);
                let error_msg = ChatMessage::System("格式錯誤".to_string());
                if let Ok(json_msg) = serde_json::to_string(&error_msg) {
                    println!("📤 发送错误消息: {}", &json_msg);
                    let _ = tx_out.send(Message::Text(json_msg));
                }
            }
        }
       } 
    });
    

    // 如果其中一個任務結束（例如使用者關掉視窗），就停止另一個任務
    tokio::select! {
        _ = (&mut send_task) => {
            recv_task.abort();
            broadcast_task.abort();
        }
        _ = (&mut recv_task) => {
            send_task.abort();
            broadcast_task.abort();
        }
        _ = (&mut broadcast_task) => {
            send_task.abort();
            recv_task.abort();
        }
    };
}
