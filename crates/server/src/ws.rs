use crate::{auth::authenticate, AppState};
use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, State, WebSocketUpgrade,
    },
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use std::{collections::HashSet, net::SocketAddr, time::Duration};
use tm_core::{Event, Id};

pub fn router() -> Router<AppState> {
    Router::new().route("/ws", get(handler))
}

/// El token NO viaja en la URL (quedaría en logs de proxies): el cliente lo envía como
/// primer mensaje de texto, `{"token": "..."}`, dentro de los primeros 5 segundos.
async fn handler(
    State(st): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let ip = crate::limits::client_ip(&headers, peer, st.trust_proxy);
    if st.ip_limiter_ws.hit(&ip, 60, Duration::from_secs(60)).is_err() {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    ws.max_message_size(2048).max_frame_size(2048).on_upgrade(move |sock| run(sock, st))
}

async fn team_ids(st: &AppState, user: Id) -> HashSet<Id> {
    sqlx::query_scalar::<_, Id>("SELECT team_id FROM team_members WHERE user_id = ?")
        .bind(user)
        .fetch_all(&st.db)
        .await
        .unwrap_or_default()
        .into_iter()
        .collect()
}

async fn handshake(sock: &mut WebSocket, st: &AppState) -> Option<Id> {
    let first = tokio::time::timeout(Duration::from_secs(5), sock.recv()).await.ok()??.ok()?;
    let Message::Text(text) = first else { return None };
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let user = authenticate(st, v.get("token")?.as_str()?).await?;
    Some(user.id)
}

async fn run(mut sock: WebSocket, st: AppState) {
    let Some(user) = handshake(&mut sock, &st).await else {
        tracing::debug!("ws: handshake rechazado");
        let _ = sock.send(Message::Close(None)).await;
        return;
    };
    if !st.ws_conns.acquire(user) {
        let _ = sock.send(Message::Close(None)).await;
        return;
    }
    let mut rx = st.tx.subscribe();
    let mut teams = team_ids(&st, user).await;
    loop {
        tokio::select! {
            msg = rx.recv() => {
                let Ok(env) = msg else { continue };
                // alta propia: suscribirse antes de filtrar; baja propia: avisar y recién después desuscribirse
                let joined = matches!(&env.event, Event::MemberAdded { user: u, .. } if u.id == user);
                let removed = matches!(&env.event, Event::MemberRemoved { user_id, .. } if *user_id == user);
                if joined { teams = team_ids(&st, user).await; }
                if !teams.contains(&env.team_id) { continue; }
                let Ok(json) = serde_json::to_string(&env.event) else { continue };
                if sock.send(Message::Text(json.into())).await.is_err() { break; }
                if removed { teams = team_ids(&st, user).await; }
            }
            incoming = sock.recv() => {
                match incoming {
                    None | Some(Err(_)) | Some(Ok(Message::Close(_))) => break,
                    _ => {}
                }
            }
        }
    }
    st.ws_conns.release(user);
}
