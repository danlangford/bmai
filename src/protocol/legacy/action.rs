// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright 2001-2026 Denis Papp
// SPDX-FileCopyrightText: Copyright 2026 Dan Langford <721364+danlangford@users.noreply.github.com>

use super::*;

impl BMC_Parser {
    pub(super) fn GetAction<W: Write>(&mut self, output: &mut W) -> Result<(), ParseError> {
        match self.m_game.m_phase {
            BME_PHASE::AUXILIARY => {
                let (die, search) = if self.m_ai_type[0] == 1 {
                    (SelectQAIAuxiliaryAction(&self.m_game), None)
                } else if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    let result = SelectNativeBMAIAuxiliaryAction(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &self.m_player_ai[0],
                    );
                    let summary = (result.score, result.probability());
                    (result.die, Some(summary))
                } else {
                    let result = SelectBMAIAuxiliaryAction(
                        &self.m_game,
                        &mut self.m_rng,
                        &self.m_player_ai[0],
                    );
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
            BME_PHASE::FIGHT => {
                if self.m_ai_type[0] != 1 {
                    let moves = self.m_game.GenerateValidAttacksInCppOrderForSearch(
                        self.m_player_ai[0].FireCandidateLimit(),
                    );
                    writeln!(
                        output,
                        "l1 p0 Valid Moves {} Sims {}",
                        moves.len(),
                        self.m_player_ai[0].ComputeNumberSims(moves.len().max(1), 1)
                    )
                    .map_err(io_error)?;
                }
                let (action, search) = if self.m_ai_type[0] == 1 {
                    (SelectQAIAction(&self.m_game, &mut self.m_rng), None)
                } else if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    let result = SelectNativeBMAIActionWithStats(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &self.m_player_ai[0],
                    );
                    let summary = (result.m_best_score, result.ProbabilityWin());
                    (result.m_move, Some(summary))
                } else {
                    let result = SelectBMAIActionWithStats(
                        &self.m_game,
                        &mut self.m_rng,
                        &self.m_player_ai[0],
                    );
                    let summary = (result.m_best_score, result.ProbabilityWin());
                    (result.m_move, Some(summary))
                };
                self.m_last_action = Some(protocol_attack(&self.m_game, &action)?);
                if let Some((best_score, probability_win)) = search {
                    writeln!(
                        output,
                        "l1 p0 best move ({:.1} points, {:.1}% win)",
                        best_score,
                        probability_win * 100.0
                    )
                    .map_err(io_error)?;
                }
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                SendAttack(&self.m_game, &action, output)
            }
            BME_PHASE::RESERVE => {
                let reserve = if self.m_ai_type[0] == 1 {
                    SelectQAIReserveAction(&self.m_game)
                } else if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    SelectNativeBMAIReserveAction(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &self.m_player_ai[0],
                    )
                } else {
                    SelectBMAIReserveAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0])
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
            BME_PHASE::PREROUND => {
                let action = if self.m_ai_type[0] == 1 {
                    SelectQAISetSwingAction(&self.m_game)
                } else if self.m_execution_mode == ExecutionMode::Native {
                    let replay = self.NextNativeReplay();
                    SelectNativeBMAISetSwingAction(
                        &self.m_game,
                        self.m_rng.Algorithm(),
                        replay,
                        self.m_native_workers,
                        &self.m_player_ai[0],
                    )
                } else {
                    SelectBMAISetSwingAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0])
                };
                self.m_last_action = Some(protocol_swing(&self.m_game, &action));
                self.SendStats(output)?;
                writeln!(output, "action").map_err(io_error)?;
                SendSetSwing(&self.m_game, &action, output)
            }
            BME_PHASE::CHANCE => {
                if self.m_ai_type[0] == 1 {
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
                        &self.m_player_ai[0],
                    )
                } else {
                    SelectBMAIChanceAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0])
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
            BME_PHASE::FOCUS => {
                if self.m_ai_type[0] == 1 {
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
                        &self.m_player_ai[0],
                    )
                } else {
                    SelectBMAIFocusAction(&self.m_game, &mut self.m_rng, &self.m_player_ai[0])
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
            self.m_max_ply, self.m_min_sims, self.m_max_sims, self.m_max_branch
        )
        .map_err(io_error)
    }
}
