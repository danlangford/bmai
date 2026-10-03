// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    BuildIdentity, Capabilities, ParseError, Parser, ProbabilityEstimate, ProtocolAction,
    ReplayMetadata, SessionMetadata,
};

pub const JSONL_PROTOCOL: &str = "jsonl-v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: String,
    id: Value,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecuteParams {
    script: String,
}

#[derive(Debug, Serialize)]
struct Response {
    protocol: &'static str,
    id: Value,
    #[serde(flatten)]
    body: ResponseBody,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum ResponseBody {
    Success { ok: bool, result: Value },
    Failure { ok: bool, error: ProtocolError },
}

#[derive(Debug, Serialize)]
struct ProtocolError {
    code: &'static str,
    message: String,
    recoverable: bool,
}

impl Response {
    fn success(id: Value, result: Value) -> Self {
        Self {
            protocol: JSONL_PROTOCOL,
            id,
            body: ResponseBody::Success { ok: true, result },
        }
    }

    fn error(id: Value, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            protocol: JSONL_PROTOCOL,
            id,
            body: ResponseBody::Failure {
                ok: false,
                error: ProtocolError {
                    code,
                    message: message.into(),
                    recoverable: true,
                },
            },
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct BmairSession {
    parser: Parser,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[non_exhaustive]
pub struct SessionExecuteResult {
    pub build: BuildIdentity,
    pub action: Option<ProtocolAction>,
    pub legacy_output: String,
    pub replay: Option<ReplayMetadata>,
    pub evaluation: Option<ProbabilityEstimate>,
    pub session: SessionMetadata,
}

impl BmairSession {
    /// A parser error leaves all prior state untouched.
    pub fn execute(&mut self, script: &str) -> Result<SessionExecuteResult, ParseError> {
        let mut candidate = self.parser.clone();
        let mut output = Vec::new();
        candidate.ParseString(script, &mut output)?;
        self.parser = candidate;
        Ok(SessionExecuteResult {
            build: BuildIdentity::current(),
            action: self.parser.last_action().cloned(),
            legacy_output: String::from_utf8(output).expect("legacy protocol output is UTF-8"),
            replay: self.parser.last_replay().cloned(),
            evaluation: self.parser.last_evaluation().cloned(),
            session: self.parser.session_metadata(),
        })
    }

    pub fn reset(&mut self) {
        self.parser = Parser::default();
    }

    pub fn metadata(&self) -> SessionMetadata {
        self.parser.session_metadata()
    }

    pub fn handle_line(&mut self, line: &str) -> String {
        let response = match serde_json::from_str::<Value>(line) {
            Ok(value) => {
                let error_id = value
                    .get("id")
                    .filter(|id| valid_id(id))
                    .cloned()
                    .unwrap_or(Value::Null);
                match serde_json::from_value::<Request>(value) {
                    Ok(request) => self.handle_request(request),
                    Err(error) => Response::error(error_id, "invalid_request", error.to_string()),
                }
            }
            Err(error) => Response::error(Value::Null, "invalid_json", error.to_string()),
        };
        serde_json::to_string(&response).expect("protocol responses are serializable")
    }

    fn handle_request(&mut self, request: Request) -> Response {
        if !valid_id(&request.id) {
            return Response::error(
                Value::Null,
                "invalid_request",
                "id must be a string, number, or null",
            );
        }
        if request.protocol != JSONL_PROTOCOL {
            return Response::error(
                request.id,
                "unsupported_protocol",
                format!(
                    "unsupported protocol: {} (expected {JSONL_PROTOCOL})",
                    request.protocol
                ),
            );
        }

        match request.method.as_str() {
            "capabilities" => {
                if !request.params.is_null() && request.params != json!({}) {
                    return Response::error(
                        request.id,
                        "invalid_params",
                        "capabilities takes no parameters",
                    );
                }
                Response::success(
                    request.id,
                    serde_json::to_value(Capabilities::current())
                        .expect("capabilities are serializable"),
                )
            }
            "session.execute" => {
                let params = match serde_json::from_value::<ExecuteParams>(request.params) {
                    Ok(params) => params,
                    Err(error) => {
                        return Response::error(request.id, "invalid_params", error.to_string());
                    }
                };

                match self.execute(&params.script) {
                    Ok(result) => Response::success(
                        request.id,
                        serde_json::to_value(result).expect("session result is serializable"),
                    ),
                    Err(error) => Response::error(request.id, "execution_error", error.to_string()),
                }
            }
            "session.reset" => {
                if !request.params.is_null() && request.params != json!({}) {
                    return Response::error(
                        request.id,
                        "invalid_params",
                        "session.reset takes no parameters",
                    );
                }
                self.reset();
                Response::success(request.id, json!({ "session": self.metadata() }))
            }
            _ => Response::error(
                request.id,
                "method_not_found",
                format!("unknown method: {}", request.method),
            ),
        }
    }
}

pub fn run_jsonl<R: BufRead, W: Write>(reader: R, mut writer: W) -> std::io::Result<()> {
    let mut session = BmairSession::default();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        writeln!(writer, "{}", session.handle_line(&line))?;
        writer.flush()?;
    }
    Ok(())
}

fn valid_id(id: &Value) -> bool {
    id.is_null() || id.is_string() || id.is_number()
}

#[cfg(test)]
mod tests;
