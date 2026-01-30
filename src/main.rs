use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

// 1. 定義共享狀態
struct AppState {
    // 廣播頻道：所有訊息都會經過這裡
    tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() {
    let (tx, _rx) = broadcast::channel(100);
    let app_store = Arc::new(AppState { tx });  

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(app_store);
    
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("🚀 聊天室已啟動：ws://127.0.0.1:3000/ws");
    axum::serve(listener,app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>
) -> impl IntoResponse{
    ws.on_upgrade(|socket| handle_socket(socket,state))
}


async fn handle_socket(socket: WebSocket, state:Arc<AppState>){
    
    let (mut sender, mut receiver) = socket.split();

    let mut rx = state.tx.subscribe();

    let _ = sender.send(Message::Text("歡迎連線".to_string())).await;


    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });
    
    let tx = state.tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = receiver.next().await {
            let _ = tx.send(text);
        }
    });

    // 如果其中一個任務結束（例如使用者關掉視窗），就停止另一個任務
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };
}
    