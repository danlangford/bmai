// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

impl Parser {
    pub(super) fn GetAction<W: Write>(&mut self, output: &mut W) -> Result<(), ParseError> {
        let player_ai = self.PlayerAI(0).clone();
        match self.m_game.m_phase {
            Phase::Auxiliary => {
                let (die, search) = if self.AIType(0) == 1 {
                    (SelectQAIAuxiliaryAction(&self.m_game), None)
                } else if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    let result = SelectNativeBMAIAuxiliaryAction(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &player_ai,
                    );
                    let summary = (result.score, result.probability());
                    (result.die, Some(summary))
                } else {
                    let result =
                        SelectBMAIAuxiliaryAction(&self.m_game, &mut self.m_rng, &player_ai);
                    let summary = (result.score, result.probability());
                    (result.die, Some(summary))
                };
                let original =
                    die.map(|index| self.m_game.m_player[0].m_die[index].m_original_index);
                self.m_last_action =
                    Some(crate::protocol::ProtocolAction::Auxiliary { die: original });
                if let Some((score, probability)) = search {
                    writeln!(
                        output,
                        "l1 p0 best move ({score:.1} points, {:.1}% win)",
                        probability * 100.0
                    )
                    .map_err(io_error)?;
                }
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                writeln!(output, "aux {}", original.map_or(-1, |index| index as i32))
                    .map_err(io_error)
            }
            Phase::Fight => {
                if self.m_report_sims > 0 && self.m_execution_mode != ExecutionMode::Native {
                    return Err(ParseError(
                        "report_sims requires native execution mode".into(),
                    ));
                }
                if self.m_report_sims > 0 && self.AIType(0) == 1 {
                    return Err(ParseError("report_sims requires BMAI search".into()));
                }
                if self.AIType(0) != 1 {
                    let moves = self
                        .m_game
                        .GenerateValidAttacksInCppOrderForSearch(player_ai.FireCandidateLimit());
                    writeln!(
                        output,
                        "l1 p0 Valid Moves {} Sims {}",
                        moves.len(),
                        player_ai.ComputeNumberSims(moves.len().max(1), 1)
                    )
                    .map_err(io_error)?;
                }
                let (action, search, report) = if self.AIType(0) == 1 {
                    (SelectQAIAction(&self.m_game, &mut self.m_rng), None, None)
                } else if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    let result = SelectNativeBMAIActionWithStats(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &player_ai,
                    );
                    let summary = (
                        result.m_best_score,
                        result.ProbabilityWin(),
                        result.m_sims_run,
                    );
                    let report = if self.m_report_sims == 0 {
                        None
                    } else {
                        let estimate = EvaluateSelectedNativeBMAIMove(
                            &self.m_game,
                            &result.m_move,
                            self.m_rng.Algorithm(),
                            replay,
                            self.m_native_workers,
                            &player_ai,
                            self.m_report_sims,
                        );
                        Some(estimate)
                    };
                    (result.m_move, Some(summary), report)
                } else {
                    let result =
                        SelectBMAIActionWithStats(&self.m_game, &mut self.m_rng, &player_ai);
                    let summary = (
                        result.m_best_score,
                        result.ProbabilityWin(),
                        result.m_sims_run,
                    );
                    (result.m_move, Some(summary), None)
                };
                self.m_last_action = Some(protocol_attack(&self.m_game, &action)?);
                if let Some((best_score, probability_win, simulations)) = search {
                    self.m_last_evaluation = Some(crate::protocol::ProbabilityEstimate {
                        player: 0,
                        probability: crate::protocol::ProtocolFloat::from_f32(probability_win),
                        simulations,
                        source: "move_selection",
                    });
                    writeln!(
                        output,
                        "l1 p0 best move ({:.1} points, {:.1}% win)",
                        best_score,
                        probability_win * 100.0
                    )
                    .map_err(io_error)?;
                }
                if let Some(estimate) = report {
                    let probability_win = estimate.ProbabilityWin();
                    self.m_last_evaluation = Some(crate::protocol::ProbabilityEstimate {
                        player: 0,
                        probability: crate::protocol::ProtocolFloat::from_f32(probability_win),
                        simulations: estimate.simulations,
                        source: "selected_move_resample",
                    });
                    writeln!(
                        output,
                        "l1 p0 selected move report ({:.1}/{}, {:.1}% win)",
                        estimate.score,
                        estimate.simulations,
                        probability_win * 100.0
                    )
                    .map_err(io_error)?;
                }
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                SendAttack(&self.m_game, &action, output)
            }
            Phase::Reserve => {
                let reserve = if self.AIType(0) == 1 {
                    SelectQAIReserveAction(&self.m_game)
                } else if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    SelectNativeBMAIReserveAction(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &player_ai,
                    )
                } else {
                    SelectBMAIReserveAction(&self.m_game, &mut self.m_rng, &player_ai)
                };
                self.m_last_action = Some(crate::protocol::ProtocolAction::Reserve {
                    die: reserve.map(|index| self.m_game.m_player[0].m_die[index].m_original_index),
                });
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                if let Some(index) = reserve {
                    writeln!(
                        output,
                        "reserve {}",
                        self.m_game.m_player[0].m_die[index].m_original_index
                    )
                    .map_err(io_error)
                } else {
                    writeln!(output, "reserve -1").map_err(io_error)
                }
            }
            Phase::Preround => {
                let action = if self.AIType(0) == 1 {
                    SelectQAISetSwingAction(&self.m_game)
                } else if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    SelectNativeBMAISetSwingAction(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &player_ai,
                    )
                } else {
                    SelectBMAISetSwingAction(&self.m_game, &mut self.m_rng, &player_ai)
                };
                self.m_last_action = Some(protocol_swing(&self.m_game, &action));
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                SendSetSwing(&self.m_game, &action, output)
            }
            Phase::Chance => {
                if self.AIType(0) == 1 {
                    self.m_last_action = Some(crate::protocol::ProtocolAction::Pass);
                    self.SendStats(output)?;
                    writeln!(output, "action\npass").map_err(io_error)?;
                    return Ok(());
                }
                let action = if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    SelectNativeBMAIChanceAction(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &player_ai,
                    )
                } else {
                    SelectBMAIChanceAction(&self.m_game, &mut self.m_rng, &player_ai)
                };
                self.m_last_action = Some(protocol_chance(&self.m_game, &action));
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                if action.reroll.is_empty() {
                    writeln!(output, "pass").map_err(io_error)
                } else {
                    for index in action.reroll {
                        writeln!(
                            output,
                            "chance {}",
                            self.m_game.m_player[0].m_die[index].m_original_index
                        )
                        .map_err(io_error)?;
                    }
                    Ok(())
                }
            }
            Phase::Focus => {
                if self.AIType(0) == 1 {
                    self.m_last_action = Some(crate::protocol::ProtocolAction::Pass);
                    self.SendStats(output)?;
                    writeln!(output, "action\npass").map_err(io_error)?;
                    return Ok(());
                }
                let action = if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    SelectNativeBMAIFocusAction(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &player_ai,
                    )
                } else {
                    SelectBMAIFocusAction(&self.m_game, &mut self.m_rng, &player_ai)
                };
                self.m_last_action = Some(protocol_focus(&self.m_game, &action));
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                if action.values.is_empty() {
                    writeln!(output, "pass").map_err(io_error)
                } else {
                    for (index, value) in action.values {
                        writeln!(
                            output,
                            "focus {} {value}",
                            self.m_game.m_player[0].m_die[index].m_original_index
                        )
                        .map_err(io_error)?;
                    }
                    Ok(())
                }
            }
            _ => Err(ParseError("GetAction(): Unrecognized phase".into())),
        }
    }

    pub(super) fn NextNativeReplay(&mut self) -> crate::native::NativeReplayKey {
        debug_assert_eq!(self.m_execution_mode, ExecutionMode::Native);
        let replay = crate::native::NativeReplayKey {
            stream_version: crate::native::NativeStreamVersion::CURRENT,
            root_seed: self.m_native_root_seed,
            decision_index: self.m_native_decision_index,
        };
        self.m_last_replay = Some(crate::protocol::ReplayMetadata {
            stream_partition: replay.stream_version.partition_id(),
            root_seed: replay.root_seed,
            decision_index: replay.decision_index,
        });
        self.m_native_decision_index = self.m_native_decision_index.wrapping_add(1);
        replay
    }

    pub(super) fn SendStats<W: Write>(&self, output: &mut W) -> Result<(), ParseError> {
        writeln!(
            output,
            "stats {}/{}-{}/{}/0.50",
            self.m_ai.m_max_ply, self.m_ai.m_min_sims, self.m_ai.m_max_sims, self.m_ai.m_max_branch
        )
        .map_err(io_error)
    }
}
