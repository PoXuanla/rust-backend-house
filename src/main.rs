use axum::{
    Router, extract::State, routing::{get, post}
};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use axum::Json;

#[derive(Deserialize, Serialize, Clone)]
struct Visitor {
    name: String,
    message: String,
}
// 1. 定義房子的「共享狀態」
// Arc: 讓每個請求（執行緒）都能擁有一份指向資料的提貨券
// Mutex: 確保同一時間只有一個人能修改人數
struct AppState {
    counter: Mutex<u32>,
    visitor_list: Mutex<Vec<Visitor>>
}

#[tokio::main]
async fn main() {
    // 2. 初始化地基：建立共享狀態
    let shared_state = Arc::new(AppState {
        counter: Mutex::new(0),
        visitor_list: Mutex::new(vec![])
    });

    // 3. 規劃房間（路由）
    let app = Router::new()
        .route("/", get(hello_world))
        .route("/visit", get(visit_house))
        .route("/register",post(register_visitor))
        .with_state(shared_state); // 把提貨券交給框架管理

    // 4. 開門營業
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    println!("🏠 房子蓋好了！地址在 http://127.0.0.1:3000");
    axum::serve(listener, app).await.unwrap();
}

// --- 房間裡的邏輯 ---

async fn hello_world() -> &'static str {
    "歡迎來到我的 Rust 之家！"
}

async fn visit_house(
    State(state): State<Arc<AppState>>, // 框架會 Clone 一份提貨券給你
) -> String {
    // 獲取鎖，把 &AppState 變成 &mut (維修工模式)
    let mut count = state.counter.lock().unwrap();
    *count += 1;
    
    format!("你是第 {} 位訪客！", count)
}

async fn register_visitor(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<Visitor>,
)-> String {
 let mut list = state.visitor_list.lock().unwrap();
 let visitor_name = payload.name.clone();
 list.push(payload);
 format!("你好 {}！你已經成功登記在名單上了。", visitor_name)
}