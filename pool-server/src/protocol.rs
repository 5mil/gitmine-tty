use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A raw inbound Stratum JSON-RPC line from the miner.
#[derive(Debug, Deserialize)]
pub struct StratumRequest {
    pub id:     Option<Value>,
    pub method: String,
    pub params: Value,
}

/// A generic outbound JSON-RPC response.
#[derive(Debug, Serialize)]
pub struct StratumResponse {
    pub id:     Value,
    pub result: Value,
    pub error:  Value,
}

impl StratumResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self { id, result, error: Value::Null }
    }
    pub fn err(id: Value, code: i64, msg: &str) -> Self {
        Self {
            id,
            result: Value::Null,
            error: serde_json::json!([code, msg, Value::Null]),
        }
    }
}

/// A server-push notification (no id).
#[derive(Debug, Serialize)]
pub struct StratumNotification {
    pub id:     Value,
    pub method: String,
    pub params: Value,
}

impl StratumNotification {
    pub fn new(method: &str, params: Value) -> Self {
        Self { id: Value::Null, method: method.to_string(), params }
    }
}

/// Parsed subscribe request.
#[derive(Debug)]
pub struct SubscribeParams {
    pub user_agent: Option<String>,
    pub session_id: Option<String>,
}

impl TryFrom<&Value> for SubscribeParams {
    type Error = ();
    fn try_from(v: &Value) -> Result<Self, ()> {
        let arr = v.as_array().ok_or(())?;
        Ok(Self {
            user_agent: arr.get(0).and_then(Value::as_str).map(str::to_string),
            session_id: arr.get(1).and_then(Value::as_str).map(str::to_string),
        })
    }
}

/// Parsed authorize request.
#[derive(Debug)]
pub struct AuthorizeParams {
    pub worker_name: String,
    pub password:    String,
}

impl TryFrom<&Value> for AuthorizeParams {
    type Error = ();
    fn try_from(v: &Value) -> Result<Self, ()> {
        let arr = v.as_array().ok_or(())?;
        let worker_name = arr.get(0).and_then(Value::as_str).ok_or(())?.to_string();
        let password    = arr.get(1).and_then(Value::as_str).unwrap_or("x").to_string();
        Ok(Self { worker_name, password })
    }
}

/// Parsed submit request.
#[derive(Debug)]
pub struct SubmitParams {
    pub worker_name:  String,
    pub job_id:       String,
    pub extranonce2:  String,
    pub ntime:        String,
    pub nonce:        String,
}

impl TryFrom<&Value> for SubmitParams {
    type Error = ();
    fn try_from(v: &Value) -> Result<Self, ()> {
        let arr = v.as_array().ok_or(())?;
        Ok(Self {
            worker_name: arr.get(0).and_then(Value::as_str).ok_or(())?.to_string(),
            job_id:      arr.get(1).and_then(Value::as_str).ok_or(())?.to_string(),
            extranonce2: arr.get(2).and_then(Value::as_str).ok_or(())?.to_string(),
            ntime:       arr.get(3).and_then(Value::as_str).ok_or(())?.to_string(),
            nonce:       arr.get(4).and_then(Value::as_str).ok_or(())?.to_string(),
        })
    }
}
