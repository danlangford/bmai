// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    BMC_Parser, Capabilities, ParseError, ProtocolAction, ReplayMetadata, SessionMetadata,
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
    parser: BMC_Parser,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SessionExecuteResult {
    pub action: Option<ProtocolAction>,
    pub legacy_output: String,
    pub replay: Option<ReplayMetadata>,
    pub session: SessionMetadata,
}

impl BmairSession {
    /// Executes a legacy command script transactionally against this session.
    /// A parser error leaves all prior state untouched.
    pub fn execute(&mut self, script: &str) -> Result<SessionExecuteResult, ParseError> {
        let mut candidate = self.parser.clone();
        let mut output = Vec::new();
        candidate.ParseString(script, &mut output)?;
        self.parser = candidate;
        Ok(SessionExecuteResult {
            action: self.parser.last_action().cloned(),
            legacy_output: String::from_utf8(output).expect("legacy protocol output is UTF-8"),
            replay: self.parser.last_replay().cloned(),
            session: self.parser.session_metadata(),
        })
    }

    pub fn reset(&mut self) {
        self.parser = BMC_Parser::default();
    }

    pub fn metadata(&self) -> SessionMetadata {
        self.parser.session_metadata()
    }

    pub fn handle_line(&mut self, line: &str) -> String {
        let response = match serde_json::from_str::<Request>(line) {
            Ok(request) => self.handle_request(request),
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
mod tests {
    use super::*;

    fn response(session: &mut BmairSession, request: Value) -> Value {
        serde_json::from_str(&session.handle_line(&request.to_string())).unwrap()
    }

    #[test]
    fn capabilities_are_discoverable_without_mutating_the_session() {
        let value = response(
            &mut BmairSession::default(),
            json!({
                "protocol": "jsonl-v1",
                "id": "python-client",
                "method": "capabilities"
            }),
        );
        assert_eq!(value["id"], "python-client");
        assert_eq!(value["ok"], true);
        assert_eq!(value["result"]["protocols"][1], "jsonl-v1");
    }

    #[test]
    fn rejected_scripts_are_transactional_and_the_session_recovers() {
        let mut session = BmairSession::default();
        let rejected = response(
            &mut session,
            json!({
                "protocol": "jsonl-v1",
                "id": 1,
                "method": "session.execute",
                "params": { "script": "workers 0" }
            }),
        );
        assert_eq!(rejected["error"]["code"], "execution_error");

        let accepted = response(
            &mut session,
            json!({
                "protocol": "jsonl-v1",
                "id": 2,
                "method": "session.execute",
                "params": { "script": "workers 2" }
            }),
        );
        assert_eq!(accepted["ok"], true);
        assert_eq!(accepted["result"]["session"]["workers"], 2);
    }

    #[test]
    fn malformed_input_does_not_end_a_multi_request_stream() {
        let input = concat!(
            "not-json\n",
            "{\"protocol\":\"jsonl-v1\",\"id\":2,\"method\":\"capabilities\"}\n"
        );
        let mut output = Vec::new();
        run_jsonl(input.as_bytes(), &mut output).unwrap();
        let responses = String::from_utf8(output).unwrap();
        let values = responses
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0]["error"]["code"], "invalid_json");
        assert_eq!(values[1]["ok"], true);
    }

    #[test]
    fn typed_actions_share_the_exact_legacy_execution_path() {
        let cases = [
            (
                "fight",
                include_str!("../tests/native-fixtures/fight.txt"),
                "attack",
            ),
            (
                "preround",
                include_str!("../tests/native-fixtures/preround.txt"),
                "set_swing",
            ),
            (
                "reserve",
                include_str!("../tests/native-fixtures/reserve.txt"),
                "reserve",
            ),
            (
                "chance",
                include_str!("../tests/native-fixtures/chance.txt"),
                "chance",
            ),
            (
                "focus",
                include_str!("../tests/native-fixtures/focus.txt"),
                "pass",
            ),
        ];

        for (id, script, action_type) in cases {
            let mut legacy_parser = BMC_Parser::default();
            let mut legacy_output = Vec::new();
            legacy_parser
                .ParseString(script, &mut legacy_output)
                .unwrap();

            let value = response(
                &mut BmairSession::default(),
                json!({
                    "protocol": "jsonl-v1",
                    "id": id,
                    "method": "session.execute",
                    "params": { "script": script }
                }),
            );
            assert_eq!(value["ok"], true, "{id}");
            assert_eq!(value["result"]["action"]["type"], action_type, "{id}");
            assert_eq!(
                value["result"]["legacy_output"],
                String::from_utf8(legacy_output).unwrap(),
                "{id}"
            );
        }
    }

    #[test]
    fn native_search_reports_the_exact_decision_replay_key() {
        let value = response(
            &mut BmairSession::default(),
            json!({
                "protocol": "jsonl-v1",
                "id": "replay",
                "method": "session.execute",
                "params": { "script": include_str!("../tests/native-fixtures/fight.txt") }
            }),
        );
        assert_eq!(value["ok"], true);
        assert_eq!(
            value["result"]["replay"]["stream_partition"],
            crate::native::NATIVE_STREAM_PARTITION_ID
        );
        assert_eq!(value["result"]["replay"]["root_seed"], 17);
        assert_eq!(value["result"]["replay"]["decision_index"], 0);
        assert_eq!(value["result"]["session"]["native_decision_index"], 1);
    }

    #[test]
    fn legacy_execution_does_not_claim_a_native_replay_key() {
        let script = include_str!("../tests/native-fixtures/fight.txt")
            .replace("mode native", "mode legacy");
        let value = response(
            &mut BmairSession::default(),
            json!({
                "protocol": "jsonl-v1",
                "id": "legacy",
                "method": "session.execute",
                "params": { "script": script }
            }),
        );
        assert_eq!(value["ok"], true);
        assert!(value["result"]["replay"].is_null());
        assert_eq!(value["result"]["session"]["native_decision_index"], 0);
    }
}
