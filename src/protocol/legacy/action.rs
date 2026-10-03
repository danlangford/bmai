// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

impl Parser {
    pub(super) fn send_action<W: Write>(&mut self, output: &mut W) -> Result<(), ParseError> {
        let player_ai = self.player_ai(0).clone();
        match self.game.phase {
            Phase::Auxiliary => {
                let (die, search) = if self.ai_type(0) == 1 {
                    (select_qai_auxiliary_action(&self.game), None)
                } else if self.execution_mode == ExecutionMode::Native {
                    let replay = self.next_native_replay();
                    let result = select_native_bmai_auxiliary_action(
                        &self.game,
                        self.rng.algorithm(),
                        replay,
                        self.native_workers,
                        &player_ai,
                    );
                    let summary = (result.score, result.probability());
                    (result.die, Some(summary))
                } else {
                    let result =
                        select_bmai_auxiliary_action(&self.game, &mut self.rng, &player_ai);
                    let summary = (result.score, result.probability());
                    (result.die, Some(summary))
                };
                let original = die.map(|index| self.game.players[0].dice[index].original_index);
                self.last_action =
                    Some(crate::protocol::ProtocolAction::Auxiliary { die: original });
                if let Some((score, probability)) = search {
                    writeln!(
                        output,
                        "l1 p0 best move ({score:.1} points, {:.1}% win)",
                        probability * 100.0
                    )
                    .map_err(io_error)?;
                }
                self.send_stats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                writeln!(output, "aux {}", original.map_or(-1, |index| index as i32))
                    .map_err(io_error)
            }
            Phase::Fight => {
                if self.report_sims > 0 && self.execution_mode != ExecutionMode::Native {
                    return Err(ParseError(
                        "report_sims requires native execution mode".into(),
                    ));
                }
                if self.report_sims > 0 && self.ai_type(0) == 1 {
                    return Err(ParseError("report_sims requires BMAI search".into()));
                }
                if self.ai_type(0) != 1 {
                    let moves = self.game.generate_valid_attacks_in_cpp_order_for_search(
                        player_ai.fire_candidate_limit(),
                    );
                    writeln!(
                        output,
                        "l1 p0 Valid Moves {} Sims {}",
                        moves.len(),
                        player_ai.compute_number_sims(moves.len().max(1), 1)
                    )
                    .map_err(io_error)?;
                }
                let (action, search, report) = if self.ai_type(0) == 1 {
                    (select_qai_action(&self.game, &mut self.rng), None, None)
                } else if self.execution_mode == ExecutionMode::Native {
                    let replay = self.next_native_replay();
                    let result = select_native_bmai_action_with_stats(
                        &self.game,
                        self.rng.algorithm(),
                        replay,
                        self.native_workers,
                        &player_ai,
                    );
                    let summary = (result.best_score, result.win_probability(), result.sims_run);
                    let report = if self.report_sims == 0 {
                        None
                    } else {
                        let estimate = evaluate_selected_native_bmai_move(
                            &self.game,
                            &result.best_move,
                            self.rng.algorithm(),
                            replay,
                            self.native_workers,
                            &player_ai,
                            self.report_sims,
                        );
                        Some(estimate)
                    };
                    (result.best_move, Some(summary), report)
                } else {
                    let result =
                        select_bmai_action_with_stats(&self.game, &mut self.rng, &player_ai);
                    let summary = (result.best_score, result.win_probability(), result.sims_run);
                    (result.best_move, Some(summary), None)
                };
                self.last_action = Some(protocol_attack(&self.game, &action)?);
                if let Some((best_score, win_probability, simulations)) = search {
                    self.last_evaluation = Some(crate::protocol::ProbabilityEstimate {
                        player: 0,
                        probability: crate::protocol::ProtocolFloat::from_f32(win_probability),
                        simulations,
                        source: "move_selection",
                    });
                    writeln!(
                        output,
                        "l1 p0 best move ({:.1} points, {:.1}% win)",
                        best_score,
                        win_probability * 100.0
                    )
                    .map_err(io_error)?;
                }
                if let Some(estimate) = report {
                    let win_probability = estimate.win_probability();
                    self.last_evaluation = Some(crate::protocol::ProbabilityEstimate {
                        player: 0,
                        probability: crate::protocol::ProtocolFloat::from_f32(win_probability),
                        simulations: estimate.simulations,
                        source: "selected_move_resample",
                    });
                    writeln!(
                        output,
                        "l1 p0 selected move report ({:.1}/{}, {:.1}% win)",
                        estimate.score,
                        estimate.simulations,
                        win_probability * 100.0
                    )
                    .map_err(io_error)?;
                }
                self.send_stats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                send_attack(&self.game, &action, output)
            }
            Phase::Reserve => {
                let reserve = if self.ai_type(0) == 1 {
                    select_qai_reserve_action(&self.game)
                } else if self.execution_mode == ExecutionMode::Native {
                    let replay = self.next_native_replay();
                    select_native_bmai_reserve_action(
                        &self.game,
                        self.rng.algorithm(),
                        replay,
                        self.native_workers,
                        &player_ai,
                    )
                } else {
                    select_bmai_reserve_action(&self.game, &mut self.rng, &player_ai)
                };
                self.last_action = Some(crate::protocol::ProtocolAction::Reserve {
                    die: reserve.map(|index| self.game.players[0].dice[index].original_index),
                });
                self.send_stats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                if let Some(index) = reserve {
                    writeln!(
                        output,
                        "reserve {}",
                        self.game.players[0].dice[index].original_index
                    )
                    .map_err(io_error)
                } else {
                    writeln!(output, "reserve -1").map_err(io_error)
                }
            }
            Phase::Preround => {
                let action = if self.ai_type(0) == 1 {
                    select_qai_set_swing_action(&self.game)
                } else if self.execution_mode == ExecutionMode::Native {
                    let replay = self.next_native_replay();
                    select_native_bmai_set_swing_action(
                        &self.game,
                        self.rng.algorithm(),
                        replay,
                        self.native_workers,
                        &player_ai,
                    )
                } else {
                    select_bmai_set_swing_action(&self.game, &mut self.rng, &player_ai)
                };
                self.last_action = Some(protocol_swing(&self.game, &action));
                self.send_stats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                send_set_swing(&self.game, &action, output)
            }
            Phase::Chance => {
                if self.ai_type(0) == 1 {
                    self.last_action = Some(crate::protocol::ProtocolAction::Pass);
                    self.send_stats(output)?;
                    writeln!(output, "action\npass").map_err(io_error)?;
                    return Ok(());
                }
                let action = if self.execution_mode == ExecutionMode::Native {
                    let replay = self.next_native_replay();
                    select_native_bmai_chance_action(
                        &self.game,
                        self.rng.algorithm(),
                        replay,
                        self.native_workers,
                        &player_ai,
                    )
                } else {
                    select_bmai_chance_action(&self.game, &mut self.rng, &player_ai)
                };
                self.last_action = Some(protocol_chance(&self.game, &action));
                self.send_stats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                if action.reroll.is_empty() {
                    writeln!(output, "pass").map_err(io_error)
                } else {
                    for index in action.reroll {
                        writeln!(
                            output,
                            "chance {}",
                            self.game.players[0].dice[index].original_index
                        )
                        .map_err(io_error)?;
                    }
                    Ok(())
                }
            }
            Phase::Focus => {
                if self.ai_type(0) == 1 {
                    self.last_action = Some(crate::protocol::ProtocolAction::Pass);
                    self.send_stats(output)?;
                    writeln!(output, "action\npass").map_err(io_error)?;
                    return Ok(());
                }
                let action = if self.execution_mode == ExecutionMode::Native {
                    let replay = self.next_native_replay();
                    select_native_bmai_focus_action(
                        &self.game,
                        self.rng.algorithm(),
                        replay,
                        self.native_workers,
                        &player_ai,
                    )
                } else {
                    select_bmai_focus_action(&self.game, &mut self.rng, &player_ai)
                };
                self.last_action = Some(protocol_focus(&self.game, &action));
                self.send_stats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                if action.values.is_empty() {
                    writeln!(output, "pass").map_err(io_error)
                } else {
                    for (index, value) in action.values {
                        writeln!(
                            output,
                            "focus {} {value}",
                            self.game.players[0].dice[index].original_index
                        )
                        .map_err(io_error)?;
                    }
                    Ok(())
                }
            }
            _ => Err(ParseError("GetAction(): Unrecognized phase".into())),
        }
    }

    pub(super) fn next_native_replay(&mut self) -> crate::native::NativeReplayKey {
        debug_assert_eq!(self.execution_mode, ExecutionMode::Native);
        let replay = crate::native::NativeReplayKey {
            stream_version: crate::native::NativeStreamVersion::CURRENT,
            root_seed: self.native_root_seed,
            decision_index: self.native_decision_index,
        };
        self.last_replay = Some(crate::protocol::ReplayMetadata {
            stream_partition: replay.stream_version.partition_id(),
            root_seed: replay.root_seed,
            decision_index: replay.decision_index,
        });
        self.native_decision_index = self.native_decision_index.wrapping_add(1);
        replay
    }

    pub(super) fn send_stats<W: Write>(&self, output: &mut W) -> Result<(), ParseError> {
        writeln!(
            output,
            "stats {}/{}-{}/{}/0.50",
            self.ai.max_ply, self.ai.min_sims, self.ai.max_sims, self.ai.max_branch
        )
        .map_err(io_error)
    }
}
